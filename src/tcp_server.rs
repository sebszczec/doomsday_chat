pub mod server {
    use tokio::net::{TcpListener, TcpStream};

    pub trait Connection {
        fn handle(self, tcp: TcpStream) -> impl std::future::Future<Output = Result<bool, bool>> + Send;
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
                    return Err(String::from(format!("Cannot start server: {}", e.to_string())));
                },
            };
    
            Ok( Self {
                server,
                connection,
            })
        }
    
        pub async fn start_loop(self) {
            loop {
                let (tcp, _) = self.server.accept().await.unwrap();
                tokio::spawn(self.connection.clone().handle(tcp));
            } 
        }
    }
}