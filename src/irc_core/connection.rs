use std::{io::Cursor, net::SocketAddr};

use tokio::{io::AsyncReadExt, net::TcpStream};
use bytes::{Buf, BytesMut};
use log::{info, warn};

use crate::irc_core::message::Message;
use crate::irc_core::message_errors::IRCError;

pub struct Connection {
    pub stream: TcpStream,
    pub address: SocketAddr,
    pub buffer: BytesMut,
}


impl Connection {
    pub fn new(stream: TcpStream, address: SocketAddr) -> Connection {
        Connection {
            stream,
            address,
            buffer: BytesMut::with_capacity(1024 * 2),
        }
    }

    pub async fn read_message(&mut self) -> Result<Option<Message>, IRCError> {
        loop {
            match self.stream.read_buf(&mut self.buffer).await {
                Ok(0) => return Err(IRCError::ClientExited),
                Ok(_n) => (),
                Err(e) => {
                    warn!("{:}", e);
                    return Err(IRCError::ClientExited);
                },
            };

            match self.parse_frame() {
                Ok(m) => return Ok(m),
                Err(_e) => (),
            }

            info!("{:?}", &self.buffer[..]);
            self.buffer.clear();
        }
    }

    // pub async fn write_message(&mut self) -> Result<(), ()>{
    //     return Err(());
    // }


    fn parse_frame(&mut self) -> Result<Option<Message>, IRCError> {
        let mut cursor = Cursor::new(&self.buffer[..]);
        match Message::check(&mut cursor) {
            Ok(_) => {
                let len = cursor.position() as usize;
                cursor.set_position(0);


                let message: Message = match Message::parse(&mut cursor) {
                    Ok(m) => m,
                    Err(IRCError::Incomplete) => return Ok(None),
                    Err(e) => return Err(e),
                };
                self.buffer.advance(len);
                self.buffer.clear();
                Ok(Some(message))
            },
            Err(IRCError::Incomplete) => Ok(None),
            Err(_e) => Err(()),
        }
    }
}
