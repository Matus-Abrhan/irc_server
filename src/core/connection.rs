use std::fmt::Result;

use tokio::net::TcpStream;
use bytes::BytesMut;

use crate::Message;

struct Connection {
    stream: TcpStream,
    buffer: BytesMut,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Connection {
        Connection { stream, buffer: BytesMut::with_capacity(2048) }
    }

    pub async fn read_message(&mut self) -> Option<Message> {
        return None;
    }

    pub async fn write_message(&mut self) {

    }
}
