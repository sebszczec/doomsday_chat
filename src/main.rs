mod chat_server;
use std::time::Duration;

use crate::chat_server::ChatConnection;
mod file_server;
use crate::file_server::FileConnection;

mod tcp_server;
use crate::tcp_server::TcpServer;

#[tokio::main]
async fn main() {
    env_logger::init();

    let tcp_server = TcpServer::new("192.168.0.123:7878", ChatConnection::new()).await.unwrap();
    tokio::spawn(tcp_server.start_loop());

    let file_server = TcpServer::new("192.168.0.123:9898", FileConnection::new()).await.unwrap();
    tokio::spawn(file_server.start_loop());

    loop {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

