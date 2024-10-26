use std::{io::Cursor, net::SocketAddr};

use tokio::{io::AsyncReadExt, io::AsyncWriteExt, net::TcpStream};
use bytes::{Buf, BytesMut};
use log::{info, warn};

use crate::irc_core::message::Message;
use crate::irc_core::error::IRCError;
use crate::irc_core::numeric::ErrorReply;

enum JoinedError {
    IRCError(IRCError),
    ErrorReply(ErrorReply),
}

pub enum RegistrationState {
    None = 0,
    PassReceived = 1,
    NickReceived = 2,
    UserReceived = 3,
    Registered = 4,
}

pub struct State {
    pub registration_state: RegistrationState,
    pub nickname: String,
    pub username: String,
    pub realname: String,
}

pub struct Connection {
    pub stream: TcpStream,
    pub address: SocketAddr,
    pub buffer: BytesMut,
    pub state: State,
}

impl Connection {
    pub fn new(stream: TcpStream, address: SocketAddr) -> Connection {
        Connection {
            stream,
            address,
            buffer: BytesMut::with_capacity(1024 * 2),
            state: State{registration_state: RegistrationState::None,
                nickname: String::new(), username: String::new(), realname: String::new()
            }
        }
    }

    pub async fn read_message(&mut self) -> Result<Option<Message>, IRCError> {
        loop {
            match self.parse_frame() {
                Ok(m) => return Ok(m),
                Err(e) => {
                    match e {
                        JoinedError::ErrorReply(e) => {
                            // return Err(e)
                            self.write_error(&e).await;
                            return Ok(None);
                        },
                        JoinedError::IRCError(e) => {
                            match e {
                                IRCError::NoMessageLeftInBuffer => (),
                                IRCError::ClientExited => panic!("This should not happen"),
                            };
                        }
                    }
                },
            }

            match self.stream.read_buf(&mut self.buffer).await {
                Ok(0) => return Err(IRCError::ClientExited),
                Ok(_n) => (),
                Err(e) => {
                    warn!("{:}", e);
                    return Err(IRCError::ClientExited);
                },
            };
            info!("received bytes: {:?}", &self.buffer[..]);
        }
    }

    pub async fn write_message(&mut self, message: &Message) {
        let mut msg_parts = message.get_parts().join(" ");
        msg_parts.push_str("\r\n");
        let bytes = msg_parts.as_bytes();

        self.stream.write_all(bytes).await.unwrap();
        // self.stream.flush().await.unwrap();
        info!("sent message: {:?}", &bytes[..]);
        self.flush_stream().await;
    }

    pub async fn flush_stream(&mut self) {
        // TODO: setup ability to queue messages
        self.stream.flush().await.unwrap();
    }

    pub async fn write_error(&mut self, error: &ErrorReply) {

        self.stream.write_i32(*error as i32).await.unwrap();
        // self.stream.flush().await.unwrap();
        info!("sent error: {:?}", *error as i32);
        self.flush_stream().await;
    }

    fn parse_frame(&mut self) -> Result<Option<Message>, JoinedError> {
        let mut cursor = Cursor::new(self.buffer.chunk());
        match Message::check(&mut cursor) {
            Ok(msg) => {
                let len = cursor.position() as usize;
                cursor.set_position(0);

                match Message::parse(msg) {
                    Ok(m) => {
                        self.buffer.advance(len);
                        // info!("Buffer remaining: {:}", self.buffer.remaining());
                        return Ok(m);
                    },
                    Err(e) => {
                        self.buffer.advance(len);
                        return Err(JoinedError::ErrorReply(e))
                    },
                };
            },
            Err(e) => return Err(JoinedError::IRCError(e)),
        };

    }
}
