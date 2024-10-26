use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use log::{info, warn, error};

use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio::time::{self, Duration};

use crate::irc_core::connection::{Connection, RegistrationState};
use crate::irc_core::command::Command;
use crate::irc_core::message::Message;
use crate::irc_core::channel::Channel;
use crate::irc_core::numeric::ErrorReply;
use crate::irc_core::error::IRCError;

// TODO: should std::Mutex or tokio Mutex be used ?
// type ChannelDb = Arc<tokio::sync::Mutex<Vec<Channel>>>;
type ChannelDb = Arc<Mutex<Vec<Channel>>>;
// NOTE: could by more granular?
// Arc<Vec<Mutex<Channel>>>

const BACKOFF_LIMIT: u64 = 64;

struct Listener {
    listener: TcpListener,
    channel_db: ChannelDb,
    notify_shutdown: broadcast::Sender<()>,
    shutdown_complete_tx: mpsc::Sender<()>,
    shutdown_complete_rx: mpsc::Receiver<()>,
}

struct Handler {
    connection: Connection,
    channel_db: ChannelDb,
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
                channel_db: self.channel_db.clone(),
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

        while !self.shutdown.is_shutdown() {
            let received = tokio::select! {
                res = self.connection.read_message() => res,
                _ = self.shutdown.recv() => {
                    info!("Server quit");
                    return Err(());
                }
            };
            match received {
                Ok(maby_message) => {
                    match maby_message {
                        Some(message) => {
                            self.write_response(message).await
                        },
                        None => {

                            (); // NOTE: No message received no response sent
                        },
                    }
                },
                Err(err) => {
                    match err {
                        IRCError::ClientExited => {
                            return Err(()); // NOTE: client exited -> end thread
                        },
                        _ => {
                            panic!("This should not happen")
                        },
                    }
                },
            };
        }
        Ok(())
    }

    async fn write_response(&mut self, message: Message) {
        match &message.command {
            Command::Pass{password} => {
                match self.connection.state.registration_state {
                    RegistrationState::None => {
                        if password == "blabla" { // Match connection password
                            self.connection.state.registration_state = RegistrationState::PassReceived;
                        } else {
                            self.connection.write_error(&ErrorReply::PasswdMismatch).await;
                        }
                    },
                    _ => {
                        self.connection.write_error(&ErrorReply::AlreadyRegistred).await;
                    },
                };
            },

            Command::Nick{nickname} => {
                let new_reg_state: Option<RegistrationState> = match self.connection.state.registration_state {
                    RegistrationState::PassReceived => {
                        Some(RegistrationState::NickReceived)
                    },
                    RegistrationState::UserReceived => {
                        Some(RegistrationState::Registered)
                    },
                    _ => {
                        None
                    },
                };
                let new_nick = nickname.to_string();
                // TODO: check if contains disallowed characters (ERR_ERRONEUSNICKNAME)
                if false {
                    self.connection.write_error(&ErrorReply::ErroneusNickname).await;
                    return;
                }
                // TODO: check if nick in useed on network (ERR_NICKNAMEINUSE)
                // TODO: ERR_NICKCOLLISION ???

                self.connection.state.nickname = new_nick;
                if let Some(reg_state) = new_reg_state {
                    self.connection.state.registration_state = reg_state;
                }
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
                        self.connection.write_error(&ErrorReply::AlreadyRegistred).await;
                    }
                }
                self.connection.state.username = user.to_string();
                self.connection.state.realname = realname.to_string();
            },

            Command::Ping{token} => {
                self.connection.write_message(&Message{
                    prefix: None,
                    command: Command::Pong{ server: None, token: token.to_string()}
                }).await
            },

            Command::Join{channels, ..} => {
                let mut channel_db = self.channel_db.lock().await;
                let mut messages: Vec<Message> = Vec::new();
                for (_idx, channel) in channels.split(',').enumerate() {
                    channel_db.push(Channel{
                        name: channel.to_string(),
                        members: Vec::from([self.connection.state.nickname.clone()]),
                        flags: Vec::new(),
                    });
                    messages.push(Message{
                        prefix: Some(self.connection.state.username.clone()),
                        command: Command::Join{channels: channel.to_string(), keys: None},
                    })
                };
                for message in messages {
                    self.connection.write_message(&message).await;
                }
                drop(channel_db);


            },
            _ => {}
        }
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
        // channel_db: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        channel_db: Arc::new(Mutex::new(Vec::new())),
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
    let listener = match TcpListener::bind("127.0.0.1:0").await {
        Ok(l) => l,
        Err(e) => panic!("{}", e),
    };
    let server_addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        run(listener, tokio::signal::ctrl_c()).await
    });

    server_addr
}

