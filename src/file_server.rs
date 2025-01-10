use crate::tcp_server::Connection;

use log::info;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use std::time::Instant;

#[derive(Clone)]
pub struct FileConnection {

}

impl FileConnection {
    pub fn new() -> Self {
        Self {}
    }
}

impl Connection for FileConnection {
    async fn handle(self, mut tcp: tokio::net::TcpStream) -> Result<bool, bool> {
        let mut file = File::open("test_input.jpg").await.unwrap();

        let mut buffer = [0u8; 1024];

        let now = Instant::now();
        let mut file_size: f32 = 0.0;

        loop {
            let bytes_read = file.read(&mut buffer).await.unwrap();
            if bytes_read == 0 {
                break;
            }

            file_size += bytes_read as f32;
            tcp.write_all(&buffer[..bytes_read]).await.unwrap();
        }

        let duration = now.elapsed();
        let mbytes = file_size / 1024.0 / 1024.0;
        let tput = mbytes / duration.as_secs_f32();

        info!("File sent, size: {:.2} mB throughput: {:.2} mB/s", mbytes, tput);   

        Ok(true)
    }

    async fn setup_broadcast(self) -> Result<bool, bool> {
        info!("No broadcast procedure");
        Ok(true)
    }

    fn get_name(&self) -> String {
        String::from("FileServer")
    }
}