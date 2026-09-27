use std::net::TcpStream;

pub struct Usuario {
    pub nombre: String,
    pub stream: TcpStream,
}
