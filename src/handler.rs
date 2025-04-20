use irc_proto::types::{Command::{self, *}, Message, Source};
use irc_proto::connection::Connection;
use log::info;
use tokio::sync::{broadcast, mpsc};

use crate::{config::CONFIG, user::{RegistrationFlags, User}};


pub struct Handler {
    pub connection: Connection,
    handler_rx: mpsc::Receiver<Message>,
    user: User,
    shutdown: broadcast::Receiver<()>,
    _running: bool,
}

impl Handler {
    pub fn new(connection: Connection, handler_rx: mpsc::Receiver<Message>, shutdown: broadcast::Receiver<()>) -> Self {
        return Handler { connection, handler_rx, user: User::new(), shutdown, _running: true }
    }

    pub async fn run(&mut self) -> Result<(), ()> {
        while self._running {
            tokio::select! {
                client_message = self.connection.read() => {
                    info!("client message");
                    match client_message {
                        Ok(msg) => self.process_message(msg).await,
                        Err(_) => return Err(()),
                    }
                }

                server_message = self.handler_rx.recv() => {
                    info!("server message");
                }

                _ = self.shutdown.recv() => {
                    info!("shutdown signal");
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
                        // TODO: add handle for client
                    }
                }
            },
            USER { user, mode, unused, realname } => {
                if self.user.register_state.contains(RegistrationFlags::PASS) {
                    self.user.username = user;
                    self.user.realname = realname;
                    self.user.register_state |= RegistrationFlags::NICK;
                    if self.user.register_state.contains(RegistrationFlags::NICK) {
                        // TODO: add handle for client
                    }
                }
            }
            PRIVMSG { targets, text } => {

            },

            _ => {},
        }

    }

}
