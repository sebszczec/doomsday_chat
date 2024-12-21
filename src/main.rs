use log::info;

mod chat_server;
use crate::chat_server::ChatConnection;

mod tcp_server;
use crate::tcp_server::TcpServer;

#[tokio::main]
async fn main() {
    env_logger::init();

    let tcp_server = TcpServer::new("192.168.0.123:7878", ChatConnection::new()).await.unwrap();
    info!("Server started");

    tcp_server.start_loop().await;
}

