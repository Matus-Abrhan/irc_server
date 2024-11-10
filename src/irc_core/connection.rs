use std::{io::Cursor, net::SocketAddr};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::{io::AsyncReadExt, io::AsyncWriteExt, net::TcpStream};
use tokio::sync::{mpsc, Mutex};
use bytes::{Buf, BytesMut};
use log::{info, warn};

use crate::irc_core::message::Message;
use crate::irc_core::command::Command;
use crate::irc_core::channel::Channel;
use crate::irc_core::numeric::{Reply, ErrorReply};
use crate::irc_core::message::Content;
use crate::irc_core::error::IRCError;


pub type ConnectionTxMap = Arc<Mutex<HashMap<String, mpsc::Sender<Message>>>>;
pub type ChannelMap = Arc<Mutex<HashMap<String, Channel>>>;

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
    pub hostname: String
}

pub struct Connection {
    pub stream: TcpStream,
    pub address: SocketAddr,
    pub connection_tx_map: ConnectionTxMap,
    pub channel_map: ChannelMap,
    pub buffer: BytesMut,
    pub state: State,
}

impl Connection {
    pub fn new(stream: TcpStream, address: SocketAddr, connection_tx_map: ConnectionTxMap, channel_map: ChannelMap) -> Connection {
        Connection {
            stream,
            address,
            connection_tx_map,
            channel_map,
            buffer: BytesMut::with_capacity(1024 * 2),
            state: State{registration_state: RegistrationState::None,
                nickname: String::new(),
                username: String::new(),
                realname: String::new(),
                hostname: String::new(),
            }
        }
    }

    pub async fn read_message(&mut self) -> Result<Option<Message>, IRCError> {
        loop {
            match self.parse_frame() {
                Ok(m) => return Ok(m),
                Err(e) => {
                    match e {
                        IRCError::NoMessageLeftInBuffer => (),
                        IRCError::ClientExited => panic!("This should not happen"),
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

    async fn _write_message(&mut self, message: &Message) {
        let mut msg_parts = message.deserialize().join(" ");
        msg_parts.push_str("\r\n");
        let bytes = msg_parts.as_bytes();

        self.stream.write_all(bytes).await.unwrap();
        info!("sent message: {:?}", &bytes[..]);
        self.flush_stream().await;
    }

    pub async fn write_command(&mut self, command: &Command) {
        self._write_message(&Message{
            prefix: None,
            content: Content::Command(command.clone()),
        }).await;
    }

    pub async fn write_error(&mut self, error: &ErrorReply) {
        self._write_message(&Message{
            prefix: None,
            content: Content::ErrorReply(error.clone()),
        }).await;
    }

    pub async fn write_reply(&mut self, reply: &Reply) {
        self._write_message(&Message{
            prefix: Some("server1".to_string()),
            content: Content::Reply(reply.clone())
        }).await;
    }

    pub async fn flush_stream(&mut self) {
        // TODO: setup ability to queue messages
        self.stream.flush().await.unwrap();
    }

    fn parse_frame(&mut self) -> Result<Option<Message>, IRCError> {
        let mut cursor = Cursor::new(self.buffer.chunk());
        match Message::check(&mut cursor) {
            Ok(msg) => {
                let len = cursor.position() as usize;
                cursor.set_position(0);

                let maby_messaage = Message::serialize(msg);
                self.buffer.advance(len);
                return Ok(maby_messaage);
            },
            Err(e) => return Err(e),
            // TODO: use ? operator
        };

    }

    pub async fn write_response(&mut self, command: &Command) {
        match &command {
            Command::Pass{password} => {
                match self.state.registration_state {
                    RegistrationState::None => {
                        if password == "blabla" { // Match connection password
                            self.state.registration_state = RegistrationState::PassReceived;
                        } else {
                            self.write_error(&ErrorReply::PasswdMismatch{client: self.state.nickname.clone()}).await;
                        }
                    },
                    _ => {
                        self.write_error(&ErrorReply::AlreadyRegistred{client: self.state.nickname.clone()}).await;
                    },
                };
            },

            Command::Nick{nickname} => {
                let new_nick = nickname.to_string();
                // TODO: check if contains disallowed characters (ERR_ERRONEUSNICKNAME)
                if false {
                    self.write_error(&ErrorReply::ErroneusNickname{client: self.state.nickname.clone(), nick: self.state.nickname.clone()}).await;
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
                        let mut connection_tx_map = self.connection_tx_map.lock().await;
                        if let Some((_, tx)) = connection_tx_map.remove_entry(&self.address.to_string()) {
                            connection_tx_map.insert(self.state.nickname.clone(), tx);
                        }
                        drop(connection_tx_map);

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
                        let mut connection_tx_map = self.connection_tx_map.lock().await;
                        if let Some((_, tx)) = connection_tx_map.remove_entry(&self.address.to_string()) {
                            connection_tx_map.insert(self.state.nickname.clone(), tx);
                        }
                        drop(connection_tx_map);

                        self.state.registration_state = RegistrationState::Registered;
                    },
                    _ => {
                        self.write_error(&ErrorReply::AlreadyRegistred{client: self.state.nickname.clone()}).await;
                    }
                }
                self.state.username = user.to_string();
                self.state.realname = realname.to_string();
            },

            Command::Ping{token} => {
                self.write_command(&Command::Pong{server: None, token: token.to_string()}).await;
            },

            Command::PrivMsg{targets, text} => {
                let target_arr: Vec<&str> = targets.split(',').collect();

                // NOTE: sned to channel targets
                // TODO: remake locking two mutexes in row
                let channel_map = self.channel_map.lock().await;
                let connection_tx_map = self.connection_tx_map.lock().await;
                for (channel_name, channel) in channel_map.iter().filter(|(k, _v)| target_arr.contains(&(*k).as_str())) {
                    for (_target, tx) in connection_tx_map.iter().filter(|(k, _v)| channel.members.contains(k) && **k != self.state.nickname) {
                        // let prefix = format!("{}!{}@{}",
                        //     self.state.nickname.to_string(),
                        //     self.state.username.to_string(),
                        //     "server1".to_string()
                        // );
                        let _ = tx.send(Message{
                            prefix: Some(self.state.nickname.to_string()),
                            // prefix: Some(prefix),
                            content: Content::Command(Command::PrivMsg{
                                targets: channel_name.to_string(),
                                text: text.to_string()
                            }),
                        }).await;
                    }
                }
                drop(connection_tx_map);
                drop(channel_map);


                // NOTE: sned to client targets
                let connection_tx_map = self.connection_tx_map.lock().await;
                for (target, tx) in connection_tx_map.iter().filter(|(k, _v)| target_arr.contains(&(*k).as_str())) {
                    let _ = tx.send(Message{
                        prefix: Some(self.state.nickname.to_string()),
                        content: Content::Command(Command::PrivMsg{
                            targets: target.to_string(),
                            text: text.to_string()
                        }),
                    }).await;
                }
                drop(connection_tx_map);
            },

            Command::Join{channels, ..} => {
                // TODO: check if user is registered

                let mut to_members: Vec<Message> = Vec::new();
                let mut channel_map = self.channel_map.lock().await;
                for channel_name in channels.split(',') {
                    match channel_map.get_mut(channel_name) {
                        Some(channel) => {
                            (*channel).members.push(self.state.nickname.clone());
                        },
                        None => {
                            assert_eq!(
                                channel_map.insert(
                                    channel_name.to_string(),
                                    Channel::new(
                                        channel_name.to_string(),
                                        self.state.nickname.clone(),
                                    )
                                ),
                                None
                            );
                        },
                    };
                    let channel = channel_map.get_mut(channel_name).unwrap();

                    // // TODO: MOTD
                    // reply_vec.push(Reply::MotdStart{client: self.state.nickname.clone(), line: "server1".to_string()});
                    // reply_vec.push(Reply::Motd{client: self.state.nickname.clone(), line: "MOTD".to_string()});
                    // reply_vec.push(Reply::EndOfMotd{client: self.state.nickname.clone()});
                    //
                    // // TODO: List of users
                    // reply_vec.push(Reply::NameReply{
                    //     client: self.state.nickname.clone(),
                    //     symbol: '@',
                    //     channel: channel.name.clone(),
                    //     members: channel.members.clone(),
                    // });
                    // reply_vec.push(Reply::EndOfNames{client: self.state.nickname.clone(), channel: channel.name.clone()})

                    to_members.push(Message{
                        prefix: Some(self.state.nickname.to_string()),
                        content: Content::Command(Command::Join{channels: channel.name.to_string(), keys: None})
                    });
                };
                warn!("{:?}", channel_map);
                drop(channel_map);

                // TODO: write to all channel memebers
                for message in to_members {
                    self._write_message(&message).await;
                }


                // TODO: write to client
                // for reply in reply_vec {
                //     self.write_reply(&reply).await;
                // }

            },

            Command::Who{mask} => {
                let mut reply_vec: Vec<Reply> = Vec::new();
                let channel_map = self.channel_map.lock().await;
                if let Some(channel) = channel_map.get(mask) {
                    for member in channel.members.iter() {
                        reply_vec.push(Reply::WhoReply{
                            client: member.to_string(),
                            channel: channel.name.to_string(),
                            username: member.to_string(),
                            host: "0.0.0.0".to_string(),
                            server: "server1".to_string(),
                            nick: member.to_string(),
                            flags: "".to_string(),
                            hopcount: "0".to_string(),
                            realname: member.to_string(),
                        })
                    }
                }
                drop(channel_map);

                for reply in reply_vec {
                    self.write_reply(&reply).await;
                }

                self.write_reply(&Reply::EndOfWho{client: self.state.nickname.to_string(), mask: mask.to_string()}).await;
            },

            _ => {},
        }
    }

    pub async fn process_client_result(&mut self, client_result: &Result<Option<Message>, IRCError>) -> Result<(), ()> {
        match client_result {
            Ok(maby_message) => {
                match maby_message {
                    Some(message) => {
                        if let Content::Command(command) = &message.content {
                            self.write_response(&command).await
                        }
                        // TODO: ErrorReply field need to be filled out
                    },
                    None => {},
                };
                return Ok(());
            },

            Err(err) => {
                match err {
                    IRCError::ClientExited => {
                        // TODO: remove from map on QUIT message?
                        let mut connection_tx_map = self.connection_tx_map.lock().await;
                        if connection_tx_map.remove(&self.address.to_string()).is_none() {
                            if connection_tx_map.remove(&self.state.nickname).is_none() {
                                warn!("This should not happen");
                            }
                        }
                        drop(connection_tx_map);

                        return Err(()); // NOTE: client exited -> end connection
                    },
                    _ => {
                        panic!("This should not happen")
                    },
                };
            },
        };
    }

    pub async fn process_server_result(&mut self, server_result: &Message) {
        // match server_result {
        //     Ok(Some(message)) => {
        //         self.write_message(&message).await;
        //     },
        //     _ => (),
        // };
        self._write_message(server_result).await;
    }
}
