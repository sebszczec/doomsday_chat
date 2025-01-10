mod chat_server;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::fs::File;
use std::time::Instant;
use log::info;

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

    let mut connection = TcpStream::connect("192.168.0.123:9898").await.unwrap();
    let mut file = File::create("test_output.jpg").await.unwrap();

    let mut buffer = [0u8; 1024];

    let now = Instant::now();
    let mut file_size: f32 = 0.0;

    loop {
        let received = connection.read(&mut buffer[..]).await.unwrap();

        if received == 0 {
            break;
        }

        file_size += received as f32;

        file.write_all(&buffer[..received]).await.unwrap();
    }

    let duration = now.elapsed();
    let mbytes = file_size / 1024.0 / 1024.0;
    let tput = mbytes / duration.as_secs_f32();

    info!("File received, size: {:.2} mB throughput: {:.2} mB/s", mbytes, tput);   

    loop {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
