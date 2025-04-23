use irc_proto::types::{Command::{self, *}, Message, Source};
use irc_proto::connection::Connection;
use log::{info, warn};
use tokio::sync::{broadcast, mpsc};

use crate::{bridge::{CommMsg, OperMsg}, config::CONFIG, user::{RegistrationFlags, User}};


pub struct Handler {
    pub connection: Connection,
    handler_rx: mpsc::Receiver<Message>,
    bridge_tx: mpsc::Sender<CommMsg>,
    channel_tx: mpsc::Sender<OperMsg>,
    user: User,
    shutdown: broadcast::Receiver<()>,
    _running: bool,
}

impl Handler {

    pub fn new(
        connection: Connection,
        bridge_tx: mpsc::Sender<CommMsg>,
        channel_tx: mpsc::Sender<OperMsg>,
        shutdown: broadcast::Receiver<()>,
    ) -> Self {
        let (_handler_tx, handler_rx) = mpsc::channel(1);
        return Handler { connection, handler_rx, bridge_tx, channel_tx, user: User::new(), shutdown, _running: true }
    }

    async fn shutdown(&mut self) {
        self.connection.shutdown().await;

    }

    pub async fn run(&mut self) -> Result<(), ()> {
        while self._running {
            tokio::select! {
                client_message = self.connection.read() => {
                    info!("client message");
                    match client_message {
                        Ok(msg) => self.process_message(msg).await,
                        Err(_) => {
                            self.shutdown().await;
                            return Err(())
                        },
                    }
                }

                server_message = self.handler_rx.recv() => {
                    match server_message {
                        Some(message) => {
                            match message.command {
                                PRIVMSG { .. } => {
                                    let _ = self.connection.write(message).await;
                                },

                                _ => {},
                            }
                        },
                        None => {},
                    }
                }

                _ = self.shutdown.recv() => {
                    info!("shutdown signal");
                    self.shutdown().await;
                    return Err(());
                },
            };
        }
        return Ok(());
    }

    async fn process_message(&mut self, msg: Message) {
        match msg.command {
            PING { token } => {
                let _ = self.connection.write(
                    Message {
                        tags: None,
                        source: Some(Source{
                                name: "server1".to_string(),
                                user: None,
                                host: None
                        }),
                        command: Command::PONG{
                            server: None, token
                        },
                    }
                ).await;
            },
            PASS { password } => {
                if self.user.register_state.is_empty() {
                    let server_passwd = CONFIG.lock().unwrap().server.password.clone();
                    if *password == server_passwd {
                        self.user.register_state |= RegistrationFlags::PASS;
                    } else {
                        let _ = self.connection.write(Message {
                            tags: None,
                            source: None,
                            command: Command::ERR_PASSWDMISMATCH {
                                client: self.user.nickname.clone(),
                            },
                        }).await;
                    }
                } else {
                    let _ =self.connection.write(Message {
                        tags: None,
                        source: None,
                        command: Command::ERR_ALREADYREGISTERED {
                            client: self.user.nickname.clone(),
                        },
                    }).await;
                }
            },
            NICK { nickname } => {
                // TODO: check if contains disallowed characters (ERR_ERRONEUSNICKNAME)
                // TODO: check if nick in useed on network (ERR_NICKNAMEINUSE)
                if self.user.register_state.contains(RegistrationFlags::PASS) {
                    self.user.nickname = nickname;
                    self.user.register_state |= RegistrationFlags::NICK;
                    if self.user.register_state.contains(RegistrationFlags::USER) {
                        let (handler_tx, handler_rx) = mpsc::channel(1);
                        self.handler_rx = handler_rx;
                        let _ = self.channel_tx.send(OperMsg::AddChannel{
                            name: self.user.username.clone(),
                            channel: handler_tx,
                        }).await;
                    }
                }
            },
            USER { user, mode, unused, realname } => {
                if self.user.register_state.contains(RegistrationFlags::PASS) {
                    self.user.username = user;
                    self.user.realname = realname;
                    self.user.register_state |= RegistrationFlags::NICK;
                    if self.user.register_state.contains(RegistrationFlags::NICK) {
                        let (handler_tx, handler_rx) = mpsc::channel(1);
                        self.handler_rx = handler_rx;
                        let _ = self.channel_tx.send(OperMsg::AddChannel{
                            name: self.user.username.clone(),
                            channel: handler_tx,
                        }).await;
                    }
                }
            }
            PRIVMSG { .. } => {
                let _ = self.bridge_tx.send(CommMsg{
                    user: self.user.clone(),
                    msg: msg.clone(),
                }).await;
            },

            _ => {},
        }

    }

}
