use crate::tcp_server::Connection;

#[derive(Clone)]
pub struct FileConnection {

}

impl FileConnection {
    pub fn new() -> Self {
        Self {}
    }
}

impl Connection for FileConnection {
    async fn handle(self, tcp: tokio::net::TcpStream) -> Result<bool, bool> {
        Ok(true)
    }

    async fn setup_broadcast(self) -> Result<bool, bool> {
        Ok(true)
    }

    fn get_name(&self) -> String {
        String::from("FileServer")
    }
}