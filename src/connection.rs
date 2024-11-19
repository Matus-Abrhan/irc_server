use std::{io::Cursor, net::SocketAddr};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::{io::AsyncReadExt, io::AsyncWriteExt, net::TcpStream};
use tokio::sync::{mpsc, Mutex};
use bytes::{Buf, BytesMut};
use log::{debug, warn};

use irc_proto::message::{Message, Content, Write};
use irc_proto::channel::Channel;
use irc_proto::command::Command;
use irc_proto::numeric::Numeric;

use crate::message::*;
use crate::error::IRCError;
use crate::config::Config;

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
    pub config: Config,
    pub connection_tx_map: ConnectionTxMap,
    pub channel_map: ChannelMap,
    pub buffer: BytesMut,
    pub buffer_out: BytesMut,
    pub state: State,
}

impl Connection {
    pub fn new(stream: TcpStream, address: SocketAddr,config: Config, connection_tx_map: ConnectionTxMap, channel_map: ChannelMap) -> Connection {
        Connection {
            stream,
            address,
            config,
            connection_tx_map,
            channel_map,
            buffer: BytesMut::with_capacity(1024 * 2),
            buffer_out: BytesMut::with_capacity(1024 * 2),
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
                        IRCError::ClientExited => panic!("This should not happen"),
                        _ => (),
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
        }
    }

    async fn write_message(&mut self, message: Message) {
        debug!("server --> {:?}: {:?}",self.state.nickname, message);
        message.write(&mut self.buffer_out);

        self.flush_stream().await;
    }

    pub async fn write_command(&mut self, command: Command) {
        self.write_message(Message{
            prefix: Some(self.config.server.name.to_string()),
            content: Content::Command(command),
        }).await;
    }

    pub async fn write_numeric(&mut self, numeric: Numeric) {
        self.write_message(Message{
            prefix: Some(self.config.server.name.to_string()),
            content: Content::Numeric(numeric),
        }).await;
    }

    pub async fn flush_stream(&mut self) {
        debug!("server --> {:?}: {:?}",self.state.nickname, &self.buffer_out[..]);
        self.stream.write_all(self.buffer_out.chunk()).await.unwrap();
        self.buffer_out.clear();
        self.stream.flush().await.unwrap();
    }

    fn parse_frame(&mut self) -> Result<Option<Message>, IRCError> {
        let mut cursor = Cursor::new(self.buffer.chunk());
        let msg_bytes = check_message(&mut cursor)?;

        debug!("{:?} --> server: {:?}",self.state.nickname, msg_bytes);
        let len = cursor.position() as usize;
        cursor.set_position(0);

        let maby_messaage = create_message(msg_bytes);
        debug!("{:?} --> server: {:?}",self.state.nickname, maby_messaage);
        self.buffer.advance(len);
        return Ok(maby_messaage);
    }

    pub async fn write_response(&mut self, command: &Command) {
        match &command {
            Command::PASS{password} => {
                match self.state.registration_state {
                    RegistrationState::None => {
                        if *password == self.config.server.password { // NOTE: Match connection password
                            self.state.registration_state = RegistrationState::PassReceived;
                        } else {
                            self.write_numeric(Numeric::ERR_PASSWDMISMATCH{client: self.state.nickname.clone()}).await;
                        }
                    },
                    _ => {
                        self.write_numeric(Numeric::ERR_ALREADYREGISTERED{client: self.state.nickname.clone()}).await;
                    },
                };
            },

            Command::NICK{nickname} => {
                let new_nick = nickname.to_string();
                // TODO: check if contains disallowed characters (ERR_ERRONEUSNICKNAME)
                if false {
                    self.write_numeric(Numeric::ERR_ERRONEUSNICKNAME{client: self.state.nickname.clone(), nick: self.state.nickname.clone()}).await;
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

            Command::USER{user, realname, ..} => {
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
                        self.write_numeric(Numeric::ERR_ALREADYREGISTERED{client: self.state.nickname.clone()}).await;
                    }
                }
                self.state.username = user.to_string();
                self.state.realname = realname.to_string();
            },

            Command::PING{token} => {
                self.write_command(Command::PONG{server: None, token: token.to_string()}).await;
            },

            Command::PRIVMSG{targets, text} => {
                let target_arr: Vec<&str> = targets.split(',').collect();

                // NOTE: sned to channel targets
                // TODO: remake locking two mutexes in row
                let channel_map = self.channel_map.lock().await;
                let filtered_channel_map = channel_map.iter().filter(|(chan_name, _chan)| target_arr.contains(&(*chan_name).as_str()));

                let connection_tx_map = self.connection_tx_map.lock().await;
                let filtered_tx_map: Vec<_> = connection_tx_map.iter().filter(|(target_name, _chan)| **target_name != self.state.nickname).collect();

                for (chan_name, chan) in filtered_channel_map {
                    for (_target_name, tx) in filtered_tx_map.iter().filter(|(target_name, _tx)| chan.members.contains(target_name)) {
                        let _ = tx.send(Message{
                            prefix: Some(self.state.nickname.to_string()),
                            content: Content::Command(Command::PRIVMSG{
                                targets: chan_name.to_string(),
                                text: text.to_string()
                            }),
                        }).await;
                    }
                }
                drop(channel_map);

                // NOTE: sned to client targets
                // TODO: should be client able to sentd message to himself ?
                for (target_name, tx) in filtered_tx_map.iter().filter(|(target_name, _chan)| target_arr.contains(&(*target_name).as_str())) {
                    let _ = tx.send(Message{
                        prefix: Some(self.state.nickname.to_string()),
                        content: Content::Command(Command::PRIVMSG{
                            targets: target_name.to_string(),
                            text: text.to_string()
                        }),
                    }).await;
                }
                drop(connection_tx_map);
            },

            Command::JOIN{channels, ..} => {
                // TODO: check if user is registered

                let mut to_members: Vec<Message> = Vec::new();
                let mut channel_map = self.channel_map.lock().await;
                for channel_name in channels.split(',') {
                    match channel_map.get_mut(channel_name) {
                        Some(channel) => {
                            (*channel).members.push(self.state.nickname.clone());
                        },
                        None => {
                            channel_map.insert(
                                channel_name.to_string(),
                                Channel::new(
                                    channel_name.to_string(),
                                    self.state.nickname.clone(),
                                )
                            );
                        },
                    };
                    let channel = channel_map.get_mut(channel_name).unwrap();

                    // TODO: MOTD
                    // reply_vec.push(Reply::MotdStart{client: self.state.nickname.clone(), line: "server1".to_string()});
                    // reply_vec.push(Reply::Motd{client: self.state.nickname.clone(), line: "MOTD".to_string()});
                    // reply_vec.push(Reply::EndOfMotd{client: self.state.nickname.clone()});

                    // TODO: List of users
                    // reply_vec.push(Reply::NameReply{
                    //     client: self.state.nickname.clone(),
                    //     symbol: '@',
                    //     channel: channel.name.clone(),
                    //     members: channel.members.clone(),
                    // });
                    // reply_vec.push(Reply::EndOfNames{client: self.state.nickname.clone(), channel: channel.name.clone()})

                    to_members.push(Message{
                        prefix: Some(self.state.nickname.to_string()),
                        content: Content::Command(Command::JOIN{channels: channel.name.to_string(), keys: None})
                    });
                };
                drop(channel_map);

                // TODO: write to all channel memebers
                for message in to_members {
                    self.write_message(message).await;
                }

                // TODO: write to client
                // for reply in reply_vec {
                //     self.write_reply(&reply).await;
                // }

            },

            Command::WHO{mask} => {
                // TODO: fill out base on config
                let mut reply_vec: Vec<Numeric> = Vec::new();
                let channel_map = self.channel_map.lock().await;
                if let Some(channel) = channel_map.get(mask) {
                    for member in channel.members.iter() {
                        reply_vec.push(Numeric::RPL_WHOREPLY{
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
                    self.write_numeric(reply).await;
                }

                self.write_numeric(Numeric::RPL_ENDOFWHO{client: self.state.nickname.to_string(), mask: mask.to_string()}).await;
            },

            _ => {},
        }
    }

    pub async fn process_client_result(&mut self, client_result: Result<Option<Message>, IRCError>) -> Result<(), ()> {
        match client_result {
            Ok(maby_message) => {
                if let Some(message) = maby_message {
                    match message.content {
                        Content::Command(command) => self.write_response(&command).await,
                        Content::Numeric(_numeric) => {},
                        Content::Unknown() => {},
                    }
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

    pub async fn process_server_result(&mut self, server_result: Message) {
        self.write_message(server_result).await;
    }
}
