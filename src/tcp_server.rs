use log::{info, error};
use tokio::net::{TcpListener, TcpStream};

pub trait Connection {
    fn handle(self, tcp: TcpStream) -> impl std::future::Future<Output = Result<bool, bool>> + Send;
    fn setup_broadcast(self) -> impl std::future::Future<Output = Result<bool, bool>> + Send;
}

pub struct TcpServer<T> {
    server: TcpListener,
    connection: T,
}

impl<T : Connection + 'static + Clone> TcpServer<T> {
    pub async fn new(address: &str, connection: T) -> Result<Self, String> {
        let server = match TcpListener::bind(address).await {
            Ok(value) => { value },
            Err(e) => { 
                error!("Cannot start server: {}", e.to_string());
                return Err(String::from(format!("Cannot start server: {}", e.to_string())));
            },
        };

        info!("Server started");

        Ok( Self {
            server,
            connection,
        })
    }

    pub async fn start_loop(self) {
        tokio::spawn(self.connection.clone().setup_broadcast());
        
        loop {
            let (tcp, _) = self.server.accept().await.unwrap();
            info!("Client connected");
            
            tokio::spawn(self.connection.clone().handle(tcp));
        } 
    }
}
