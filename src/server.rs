use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use log::{info, warn};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc};
use tokio::time::{self, Duration};

use crate::irc_core::connection::{Connection, RegistrationState};
use crate::irc_core::command::Command;
use crate::irc_core::message::Message;
use crate::irc_core::message_errors::IRCError;

type Db = Arc<tokio::sync::Mutex<Vec<String>>>;
const BACKOFF_LIMIT: u64 = 64;

struct Listener {
    messages: Db,
    nicks: Db,
    listener: TcpListener,
    notify_shutdown: broadcast::Sender<()>,
    shutdown_complete_tx: mpsc::Sender<()>,
    shutdown_complete_rx: mpsc::Receiver<()>,
}

struct Handler {
    messages: Db,
    nicks: Db,
    connection: Connection,
    shutdown: Shutdown,
    _shutdown_complete: mpsc::Sender<()>,
}

struct Shutdown {
    shutdown: bool,
    notify: broadcast::Receiver<()>
}

impl Listener {
    async fn run(&mut self) -> Result<(), ()> {

        loop {
            let (stream, address) = self.accept().await?;
            let mut handler = Handler{
                // db: self.db.clone(),
                messages: self.messages.clone(),
                nicks: self.nicks.clone(),
                connection: Connection::new(stream, address),
                shutdown: Shutdown::new(self.notify_shutdown.subscribe()),
                _shutdown_complete: self.shutdown_complete_tx.clone(),
            };
            info!("{:} connected", handler.connection.address);

            tokio::spawn(async move {
                if (handler.run().await).is_err() {
                    info!("{:} exited", handler.connection.address);
                }
            });
        }
    }

    async fn accept(&mut self) -> Result<(TcpStream, SocketAddr), ()> {
        let mut backoff = 1;
        loop {
            match self.listener.accept().await {
                Ok((stream, addr)) => return Ok((stream, addr)),
                Err(err) => {
                    if backoff > BACKOFF_LIMIT {
                        warn!("{}", err);
                        return Err(());
                    }
                }
            }
            time::sleep(Duration::from_secs(backoff)).await;
            backoff *= 2;
        }
    }
}

impl Handler {
    async fn run(&mut self) -> Result<(), ()> {

        let messages = self.messages.lock().await;
        for msg in messages.iter() {
            self.connection.stream.write_all(msg.as_bytes()).await.unwrap();
        }
        drop(messages);

        while !self.shutdown.is_shutdown() {
            let received = tokio::select! {
                res = self.connection.read_message() => res,
                _ = self.shutdown.recv() => {
                    self.connection.stream.write_all("server quit\n".as_bytes()).await.unwrap();
                    return Err(());
                }
            };
            self.connection.buffer.clear();
            let maby_response: Option<String> = match received {
                Ok(maby_message) => {
                    match maby_message {
                        Some(message) => {
                            match self.generate_response(message).await {
                                Ok(maby_response) => {
                                    match maby_response {
                                        Some(response) => {
                                            Some(response.to_string())
                                        },
                                        None => None,
                                    }
                                },
                                Err(err) => {
                                    // warn!("{:?}", err);
                                    Some((err as i32).to_string())
                                }
                            }
                        },
                        None => None,
                    }
                },
                Err(err) => {
                    match err {
                        IRCError::ClientExited => {
                            return Err(());
                        },
                        _ => {
                            // warn!("{:?}", err);
                            Some((err as i32).to_string())
                        },
                    }
                },
            };

            if let Some(mut resp) = maby_response {
                resp.push('\n');
                self.connection.stream.write_all(resp.as_bytes()).await.unwrap();

                let mut messages = self.messages.lock().await;
                messages.push(resp);
                drop(messages);
            }

        }
        Ok(())
    }

    async fn generate_response(&mut self, message: Message) -> Result<Option<Message>, IRCError> {
        let respoonse_command: Command = match &message.command {
            Command::Pass{password} => {
                match self.connection.state.registration_state {
                    RegistrationState::None => {
                        if password == "blabla" { // Match connection password
                            self.connection.state.registration_state = RegistrationState::PassReceived;
                            return Ok(None);
                        } else {
                            // warn!("{:?}", IRCError::PasswdMismatch);
                            return Err(IRCError::PasswdMismatch);
                        }
                    },
                    _ => {
                        // warn!("{:?}", IRCError::AlreadyRegistred);
                        // return Err(IRCError::AlreadyRegistred);
                        return Ok(None)
                    }
                }
            },
            Command::Nick{nickname} => {
                let new_reg_state = match self.connection.state.registration_state {
                    RegistrationState::PassReceived => {
                        RegistrationState::UserReceived
                    },
                    RegistrationState::UserReceived => {
                        RegistrationState::Registered
                    },
                    _ => {
                        // warn!("{:?}", IRCError::AlreadyRegistred);
                        return Err(IRCError::AlreadyRegistred);
                    }
                };
                let new_nick = nickname.to_string();
                // TODO: check if contains disallowed characters (ERR_ERRONEUSNICKNAME)
                if false {
                    return Err(IRCError::ErroneusNickname);
                }
                // TODO: check if nick in useed on network (ERR_NICKNAMEINUSE)
                let mut nicks = self.nicks.lock().await;
                if nicks.contains(&new_nick) {
                    return Err(IRCError::NicknameInUse);
                }
                nicks.push(new_nick.clone());
                drop(nicks);
                // TODO: ERR_NICKCOLLISION ???

                self.connection.state.nickname = new_nick;
                self.connection.state.registration_state = new_reg_state;
                return Ok(None);
            },
            Command::User{user, realname, ..} => {
                match self.connection.state.registration_state {
                    RegistrationState::PassReceived => {
                        self.connection.state.registration_state = RegistrationState::UserReceived;
                    },
                    RegistrationState::NickReceived=> {
                        self.connection.state.registration_state = RegistrationState::Registered;
                    },
                    _ => {
                        warn!("{:?}", IRCError::AlreadyRegistred);
                        return Err(IRCError::AlreadyRegistred);
                    }
                }
                self.connection.state.username = user.to_string();
                self.connection.state.realname = realname.to_string();
                return Ok(None);
            },
            Command::Ping{token} => {
                Command::Pong{ server: None, token: token.to_string()}
            },
            _ => todo!("Add all cases")

        };

        let response_message = Message{prefix: Some(":Server".to_string()), command: respoonse_command};

        return Ok(Some(response_message));
    }
}

impl Shutdown {
    fn new(notify: broadcast::Receiver<()>) -> Shutdown {
        Shutdown {shutdown: false, notify}
    }

    fn is_shutdown(&self) -> bool {
        self.shutdown
    }

    async fn recv(&mut self) {
        if self.shutdown {
            return;
        }

        let _ = self.notify.recv().await;
        self.shutdown = true
    }
}

pub async fn run(listener: TcpListener, shutdown: impl Future) -> Result<(), ()> {
    let (notify_shutdown, _) = broadcast::channel(1);
    let (shutdown_complete_tx, shutdown_complete_rx) = mpsc::channel(1);

    let mut server = Listener {
        listener,
        messages: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        nicks: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        notify_shutdown,
        shutdown_complete_tx,
        shutdown_complete_rx
    };
    tokio::select! {
        _ = server.run() => {}
        _ = shutdown => {}
    }

    let Listener{
        mut shutdown_complete_rx,
        shutdown_complete_tx,
        notify_shutdown,
        ..
    } = server;
    drop(notify_shutdown);
    drop(shutdown_complete_tx);
    let _ = shutdown_complete_rx.recv().await;

    Ok(())
}

pub async fn start_server() -> SocketAddr {
    let server_addr: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1234);

    let listener = match TcpListener::bind(server_addr).await {
        Ok(l) => l,
        Err(e) => panic!("{}", e),
    };
    tokio::spawn(async move {
        run(listener, tokio::signal::ctrl_c()).await
    });

    server_addr
}

