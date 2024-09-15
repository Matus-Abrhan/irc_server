use std::{io::Cursor, net::SocketAddr};

use tokio::{io::AsyncReadExt, net::TcpStream};
use bytes::{Buf, BytesMut};
use log::info;

use super::message::{Message, Error};

pub struct Connection {
    pub stream: TcpStream,
    pub address: SocketAddr,
    pub buffer: BytesMut,
}

pub enum ConnectionError {
    Exited,
    Other,
}

impl Connection {
    pub fn new(stream: TcpStream, address: SocketAddr) -> Connection {
        Connection {
            stream,
            address,
            buffer: BytesMut::with_capacity(1024 * 2),
        }
    }

    pub async fn read_message(&mut self) -> Result<Option<Message>, ConnectionError> {
        loop {
            match self.stream.read_buf(&mut self.buffer).await {
                Ok(0) => return Err(ConnectionError::Exited),
                Ok(_n) => (),
                Err(_e) => return Err(ConnectionError::Other),
            };

            // if let Ok(result) = self.parse_frame() {
            //     return Ok(result)
            // };

            let mut cursor = Cursor::new(&self.buffer[..]);
            cursor.set_position(0);
            match Message::parse(&mut cursor) {
                Ok(m) => info!("{:?}", m),
                Err(_e) => (),
            };
            info!("{:?}", &self.buffer[..]);

            self.buffer.clear();
        }
    }

    // pub async fn write_message(&mut self) -> Result<(), ()>{
    //     return Err(());
    // }


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
                Ok(Some(message))
            },
            Err(Error::Incomplete) => Ok(None),
            Err(_e) => Err(()),
        }

        // let len = cursor.position() as usize;
        // cursor.set_position(0);
        //
        //
        // let message: Message = match Message::parse(&mut cursor) {
        //     Ok(m) => m,
        //     // Err(Error::Incomplete) => return Ok(None),
        //     Err(Error::Incomplete) => return Err(()),
        //     Err(_e) => return Err(()),
        // };
        // // self.buffer.advance(len);
        // self.buffer.clear();
        // Ok(Some(message))
    }
}
