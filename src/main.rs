use std::collections::HashMap;
use std::{collections::HashSet, sync::Arc};
use std::sync::RwLock;
use tokio::{net::{TcpListener, TcpStream}, sync::broadcast};
use log::{info, debug, warn, error};
use futures::{SinkExt, StreamExt};
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};
use tokio::sync::broadcast::{Receiver, Sender};

use crate::random_names::name_generator::Names;
mod random_names;

const HELP_MSG: &str = include_str!("help.txt");

struct Room {
    tx: Sender<String>,
    users: HashSet<String>,
}

impl Room {
    fn new() -> Self {
        let (tx, _) = broadcast::channel(32);
        let users = HashSet::new();
        Self {
            tx,
            users,
        }
    }
}

const MAIN: &str = "main";

#[derive(Clone)]
struct Rooms(Arc<RwLock<HashMap<String, Room>>>);

impl Rooms {
    fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }

    fn join(&self, room_name: &str, user_name: &str) -> Sender<String> {
        let mut write_guard = self.0.write().unwrap();
        let room = write_guard.entry(room_name.to_owned()).or_insert(Room::new());
        room.users.insert(user_name.to_owned());
        room.tx.clone()
    }

    fn leave(&self, room_name: &str, user_name: &str) {
        let mut write_guard = self.0.write().unwrap();
        let mut delete_room = false;
        if let Some(room) = write_guard.get_mut(room_name) {
            room.users.remove(user_name);
            delete_room = room.tx.receiver_count() <= 1;
        }
        if delete_room {
            write_guard.remove(room_name);
        }
    }

    fn change(&self, prev_room: &str, next_room: &str, user_name: &str) -> Sender<String> {
        self.leave(prev_room, user_name);
        self.join(next_room, user_name)
    }

    fn list(&self) -> Vec<(String, usize)> {
        let mut list: Vec<_> = self.0
            .read()
            .unwrap()
            .iter()
            .map(|(name, room)| (
                name.to_owned(),
                room.tx.receiver_count(),
            ))
            .collect();

        list.sort_by(|a, b| {
            use std::cmp::Ordering::*;
            // sort rooms by # of users first
            match b.1.cmp(&a.1) {
                // and by alphabetical order second
                Equal => a.0.cmp(&b.0),
                ordering => ordering,
            }
        });

        list
    }

    fn list_users(&self, room_name: &str) -> Option<Vec<String>> {
        self
            .0
            .read()
            .unwrap()
            .get(room_name)
            .map(|room| {
                let mut users = room
                    .users
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>();
                users.sort();
                users
            })
    }
}

struct TcpContext<'a> {
    stream: FramedRead<tokio::net::tcp::ReadHalf<'a>, LinesCodec>,
    sink: FramedWrite<tokio::net::tcp::WriteHalf<'a>, LinesCodec>,
}

impl<'a> TcpContext<'a> {
    fn new(tcp: &'a mut TcpStream) -> Self {
        let (reader, writer) = tcp.split();

        Self {
            sink : FramedWrite::new(writer, LinesCodec::new()),
            stream : FramedRead::new(reader, LinesCodec::new()),
        }
    }
}

struct ClientContext {
    name: String,
    room_name: String,
    room_tx: Sender<String>,
    room_rx: Receiver<String>,
}

impl ClientContext {
    fn new(name: String, room_name: String, room_tx: Sender<String>, room_rx: Receiver<String>) -> Self {
        Self { 
            name, 
            room_name,
            room_tx,
            room_rx,
        }
    }
}

#[derive(Debug)]
enum CommandError {
    Quit,
    WrongCommand,
    NotEnoughArg,
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Quit => write!(f, "Quit command received"),
            Self::WrongCommand => write!(f, "Wrong command received"),
            Self::NotEnoughArg => write!(f, "Command with not enough number of arguments"),
        }
    }
}

fn take_first_argument(command: &String) -> Result<&str, CommandError> {
    match command.split_ascii_whitespace().nth(1) {
        Some(value) => {
            return Ok(value);
        }
        None => {
            return Err(CommandError::NotEnoughArg);
        }
    }
}


pub trait Connection {
    fn handle(self, tcp: TcpStream) -> impl std::future::Future<Output = Result<bool, bool>> + Send;
}

#[derive(Clone)]
pub struct ChatConnection {
    names : Names,
    rooms : Rooms
}

impl ChatConnection {
    fn new() -> Self {
        Self {
            names : Names::new(),
            rooms : Rooms::new()
        }
    }

    async fn handle_command(command: &String, tcp_context: &mut TcpContext<'_>, context: &mut ClientContext, rooms: Rooms, names: Names) -> Result<bool, CommandError> {
        if command.starts_with("/quit") {
            return Err(CommandError::Quit);
        }
        else if command.starts_with("/help") {
            tcp_context.sink.send(HELP_MSG).await.unwrap();
        }
        else if command.starts_with("/rooms") {
            let rooms_list = rooms.list();
            let rooms_list = rooms_list
                .into_iter()
                .map(|(name, count)| format!("{name} ({count})"))
                .collect::<Vec<_>>()
                .join(", ");
            tcp_context.sink.send(format!("Rooms - {rooms_list}")).await.unwrap();
        }
        else if command.starts_with("/join") {
            let new_room;
            match take_first_argument(command) {
                std::result::Result::Ok(value) => {
                    new_room = value.to_owned();
                },
                Err(e) => {
                    return Err(e);
                }
            }

            if new_room == context.room_name {
                tcp_context.sink.send(format!("You already are in {}", context.room_name)).await.unwrap();
                return Ok(true)
            }

            context.room_tx.send(format!("{} has left {}", context.name, context.room_name)).unwrap();
            context.room_tx = rooms.change(&context.room_name, &new_room, &context.name);
            context.room_rx = context.room_tx.subscribe();
            context.room_name = new_room;
            context.room_tx.send(format!("{} joined {}", context.name, context.room_name)).unwrap();
        }
        else if command.starts_with("/users") {
            let users_list = rooms.list_users(&context.room_name).unwrap().join(", ");
            tcp_context.sink.send(format!("Users - {users_list}")).await.unwrap();
        }
        else if command.starts_with("/name") {
            let new_name;
            match take_first_argument(command) {
                std::result::Result::Ok(value) => {
                    new_name = value.to_owned();
                },
                Err(e) => {
                    return Err(e);
                }
            }

            let changed_name = names.insert(new_name.clone());
            if changed_name {
                context.room_tx.send(format!("{} is now {}", context.name, new_name)).unwrap();
                names.remove(&context.name);

                rooms.leave(&context.room_name, &context.name);
                rooms.join(&context.room_name, new_name.as_str());

                context.name = new_name;
                
            } else {
                tcp_context.sink.send(format!("{new_name} is already taken")).await.unwrap();
            }
        }
        else {
            return Err(CommandError::WrongCommand);
        }

        Ok(true)
    }

}

impl Connection for ChatConnection {
    async fn handle(self, mut tcp: TcpStream) -> Result<bool, bool> {
        let mut tcp_context = TcpContext::new(&mut tcp);
        tcp_context.sink.send(HELP_MSG).await.unwrap();

        let user_name = self.names.get_unique();
        let room_name = MAIN.to_owned();

        let room_tx = self.rooms.join(&room_name, &user_name);

        let mut context = ClientContext::new(
            user_name, 
            room_name,
            room_tx.clone(),
            room_tx.subscribe());
        tcp_context.sink.send(format!("Your name is {}", context.name)).await.unwrap();
        
        let _ = context.room_tx.send(format!("{} joined {}", context.name, context.room_name));

        let result = loop {
            tokio::select! {
                user_msg = tcp_context.stream.next() => {
                    let user_msg = match user_msg {
                        Some(msg) => msg.unwrap(),
                        None => { break Ok::<bool, bool>(true)},
                    };

                    if user_msg.starts_with("/") {
                        match Self::handle_command(&user_msg, &mut tcp_context, &mut context, self.rooms.clone(), self.names.clone()).await {
                            std::result::Result::Ok(_) => { continue; },
                            Err(e) => match e {
                                CommandError::WrongCommand => { 
                                    tcp_context.sink.send(format!("Wrong command: {user_msg}")).await.unwrap();
                                    continue; 
                                },
                                CommandError::Quit => { break Ok(true); },
                                CommandError::NotEnoughArg => {
                                    tcp_context.sink.send(format!("Command with wrong number of arguments: {user_msg}")).await.unwrap();
                                    continue; 
                                }
                            },
                        }
                    }
                    else {
                        let _ = context.room_tx.send(format!("{}: {}", context.name, user_msg));
                    }
                },
                peer_msg = context.room_rx.recv() => {
                    tcp_context.sink.send(peer_msg.unwrap()).await.unwrap();
                },
            }
        };
        
        info!("Client disconnected");
        let _ = context.room_tx.send(format!("{} has left {}", context.name, context.room_name));
        self.rooms.leave(&context.room_name, &context.name);
        self.names.remove(&context.name);

        result
    }
}

struct TcpServer<T> {
    server: TcpListener,
    connection: T,
}

impl<T : Connection + 'static + Clone> TcpServer<T> {
    async fn new(address: &str, connection: T) -> Result<Self, String> {
        let server = match TcpListener::bind(address).await {
            Ok(value) => { value },
            Err(e) => { 
                error!("Cannot start server: {}", e.to_string());
                return Err(String::from(format!("Cannot start server: {}", e.to_string())));
            },
        };

        Ok( Self {
            server,
            connection,
        })
    }

    async fn start_loop(self) {
        loop {
            let (tcp, _) = self.server.accept().await.unwrap();
            info!("Client connected");
            
            tokio::spawn(self.connection.clone().handle(tcp));
            //tokio::spawn(TcpServer::<T>::handle_user(tcp,self.names.clone(), self.rooms.clone()));
        } 
    }
}

pub trait Interface1 {
    fn foo_bar(self);
}

struct Implementation1;
impl Interface1 for Implementation1 {
    fn foo_bar(self) {
        info!("---> Success1 <---");
    }
}

struct TestStruct1<T> {
    t : T,
}

impl<T : Interface1> TestStruct1<T>
{
    fn new(a : T) -> Self {
        Self { t : a}
    }
    fn test_bar(self) {
        info!("---> Almost Success1 <---");
        self.t.foo_bar();
    }
}

pub trait Interface2 {
    fn foo_bar();
}

struct Implementation2;
impl Interface2 for Implementation2 {
    fn foo_bar() {
        info!("---> Success2 <---");
    }
}

use std::marker::PhantomData;
struct TestStruct2<T> {
    phantom: PhantomData<T>,
}

impl<T : Interface2> TestStruct2<T>
{
    fn test_bar() {
        info!("---> Almost Success2 <---");
        T::foo_bar();
    }
}



#[tokio::main]
async fn main() {
    env_logger::init();

    warn!("WARN test");
    error!("ERROR test");
    info!("INFO test");
    debug!("DEBUG test");

    let test_impl = Implementation1;
    let test_struct = TestStruct1::<Implementation1>::new(test_impl);
    test_struct.test_bar();

    TestStruct2::<Implementation2>::test_bar();

    let tcp_server = TcpServer::new("192.168.0.123:7878", ChatConnection::new()).await.unwrap();
    info!("Server started");

    tcp_server.start_loop().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrong_number_of_arguments() {
        let command = String::from("/name");
        match take_first_argument(&command) {
            std::result::Result::Ok(_) => {
                assert!(false);
            },
            Err(_) => {
                assert!(true);
            }

        }
    }

    #[test]
    fn correct_number_of_arguments() {
        let command = String::from("/name JohnWick");
        match take_first_argument(&command) {
            std::result::Result::Ok(value) => {
                assert_eq!("JohnWick", value);
            },
            Err(_) => {
                assert!(false);
            }

        }
    }

}
