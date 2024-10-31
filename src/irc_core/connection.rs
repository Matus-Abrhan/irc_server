use std::{io::Cursor, net::SocketAddr};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::{io::AsyncReadExt, io::AsyncWriteExt, net::TcpStream};
use tokio::sync::{mpsc, Mutex};
use bytes::{Buf, BytesMut};
use log::{info, warn};

use crate::irc_core::message::Message;
use crate::irc_core::command::Command;
use crate::irc_core::error::IRCError;
use crate::irc_core::numeric::ErrorReply;

enum JoinedError {
    IRCError(IRCError),
    ErrorReply(ErrorReply),
}

#[derive(Clone, Copy)]
pub enum RegistrationState {
    None = 0,
    PassReceived = 1,
    NickReceived = 2,
    UserReceived = 3,
    Registered = 4,
}

#[derive(Clone)]
pub struct State {
    pub registration_state: RegistrationState,
    pub nickname: String,
    pub username: String,
    pub realname: String,
}

pub struct Connection {
    pub stream: TcpStream,
    pub address: SocketAddr,
    pub task_sender_map: Arc<Mutex<HashMap<String, mpsc::Sender<Result<Option<Message>, IRCError>>>>>,
    pub buffer: BytesMut,
    pub state: State,
}

impl Connection {
    pub fn new(stream: TcpStream, address: SocketAddr, task_sender_map: Arc<Mutex<HashMap<String, mpsc::Sender<Result<Option<Message>, IRCError>>>>>) -> Connection {
        Connection {
            stream,
            address,
            task_sender_map,
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

    pub async fn write_response(&mut self, message: Message) {
        match &message.command {
            Command::Pass{password} => {
                match self.state.registration_state {
                    RegistrationState::None => {
                        if password == "blabla" { // Match connection password
                            self.state.registration_state = RegistrationState::PassReceived;
                        } else {
                            self.write_error(&ErrorReply::PasswdMismatch).await;
                        }
                    },
                    _ => {
                        self.write_error(&ErrorReply::AlreadyRegistred).await;
                    },
                };
            },

            Command::Nick{nickname} => {
                let new_nick = nickname.to_string();
                // TODO: check if contains disallowed characters (ERR_ERRONEUSNICKNAME)
                if false {
                    self.write_error(&ErrorReply::ErroneusNickname).await;
                    return;
                }
                // TODO: check if nick in useed on network (ERR_NICKNAMEINUSE)
                // TODO: ERR_NICKCOLLISION ???

                self.state.nickname = new_nick;
                let new_reg_state: Option<RegistrationState> = match self.state.registration_state {
                    RegistrationState::PassReceived => {
                        Some(RegistrationState::NickReceived)
                    },
                    RegistrationState::UserReceived => {
                        let mut task_sender_map = self.task_sender_map.lock().await;
                        if let Some((_, tx)) = task_sender_map.remove_entry(&self.address.to_string()) {
                            task_sender_map.insert(self.state.nickname.clone(), tx);
                        }
                        drop(task_sender_map);

                        Some(RegistrationState::Registered)
                    },
                    _ => {
                        None
                    },
                };

                if let Some(reg_state) = new_reg_state {
                    self.state.registration_state = reg_state;
                }
            },

            Command::User{user, realname, ..} => {
                match self.state.registration_state {
                    RegistrationState::PassReceived => {
                        self.state.registration_state = RegistrationState::UserReceived;
                    },
                    RegistrationState::NickReceived=> {
                        let mut task_sender_map = self.task_sender_map.lock().await;
                        if let Some((_, tx)) = task_sender_map.remove_entry(&self.address.to_string()) {
                            task_sender_map.insert(self.state.nickname.clone(), tx);
                        }
                        drop(task_sender_map);

                        self.state.registration_state = RegistrationState::Registered;
                    },
                    _ => {
                        self.write_error(&ErrorReply::AlreadyRegistred).await;
                    }
                }
                self.state.username = user.to_string();
                self.state.realname = realname.to_string();
            },

            Command::Ping{token} => {
                self.write_message(&Message{
                    prefix: None,
                    command: Command::Pong{ server: None, token: token.to_string()}
                }).await
            },

            Command::PrivMsg{targets, text} => {
                let task_sender_map = self.task_sender_map.lock().await;
                let target_arr: Vec<&str> = targets.split(',').collect();
                for (target, channel) in task_sender_map.iter().filter(|(k, _v)| target_arr.contains(&(*k).as_str())) {
                    let _ = channel.send(Ok(Some(Message{
                        prefix: Some(":".to_owned()+&self.state.nickname),
                        command: Command::PrivMsg{
                            targets: target.to_string(),
                            text: text.to_string()
                        },
                    }))).await;
                }
                drop(task_sender_map)
            },

            // Command::Join{channels, ..} => {
            //     let mut channel_db = self.channel_db.lock().await;
            //     let mut messages: Vec<Message> = Vec::new();
            //     for (_idx, channel) in channels.split(',').enumerate() {
            //         channel_db.push(Channel{
            //             name: channel.to_string(),
            //             members: Vec::from([self.connection.state.nickname.clone()]),
            //             flags: Vec::new(),
            //         });
            //         messages.push(Message{
            //             prefix: Some(self.connection.state.username.clone()),
            //             command: Command::Join{channels: channel.to_string(), keys: None},
            //         })
            //     };
            //     for message in messages {
            //         self.connection.write_message(&message).await;
            //     }
            //     drop(channel_db);
            //
            //
            // },

            _ => {},
        }
    }
}
