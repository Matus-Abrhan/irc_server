use std::{io::Cursor, net::SocketAddr};

use tokio::{io::AsyncReadExt, net::TcpStream};
use bytes::{Buf, BytesMut};
use log::info;

use super::message::{Message, Error};

pub struct Connection {
    pub stream: TcpStream,
    // address: SocketAddr,
    pub buffer: BytesMut,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Connection {
        Connection {
            stream,
            // address,
            buffer: BytesMut::with_capacity(1024 * 2),
        }
    }

    pub async fn read_message(&mut self) -> Result<Option<Message>, ()> {
        loop {
            if let Some(message) = self.parse_frame()? {
                return Ok(Some(message));
            }
            if 0 == self.stream.read_buf(&mut self.buffer).await.unwrap() {
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    return Err(());
                }
            }

            // let n = match self.stream.read_buf(&mut self.buffer).await {
            //     Ok(n) => n,
            //     Err(_) => panic!("FUUUCK"),
            // };
            info!("{:?}", &self.buffer[..]);
        }
    }

    pub async fn write_message(&mut self) -> Result<(), ()>{
        return Err(());
    }


    fn parse_frame(&mut self) -> Result<Option<Message>, ()> {
        let mut cursor = Cursor::new(&self.buffer[..]);
        match Message::check(&mut cursor) {
            Ok(_) => {
                let len = cursor.position() as usize;
                cursor.set_position(0);


                let message: Message = match Message::parse(&mut cursor) {
                    Ok(m) => m,
                    Err(Error::Incomplete) => return Ok(None),
                    Err(_e) => return Err(()),
                };
                self.buffer.advance(len);
                self.buffer.clear();
                return Ok(Some(message));
            },
            Err(Error::Incomplete) => Ok(None),
            Err(_e) => Err(()),
        }
    }
}
