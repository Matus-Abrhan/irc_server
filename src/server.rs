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
    db: Db,
    listener: TcpListener,
    notify_shutdown: broadcast::Sender<()>,
    shutdown_complete_tx: mpsc::Sender<()>,
    shutdown_complete_rx: mpsc::Receiver<()>,
}

struct Handler {
    db: Db,
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
                db: self.db.clone(),
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


        let db = self.db.lock().await;
        for msg in db.iter() {
            self.connection.stream.write_all(msg.as_bytes()).await.unwrap();
        }
        drop(db);

        while !self.shutdown.is_shutdown() {
            let received = tokio::select! {
                res = self.connection.read_message() => res,
                _ = self.shutdown.recv() => {
                    self.connection.stream.write_all("server quit\n".as_bytes()).await.unwrap();
                    return Err(());
                }
            };
            // info!("{:?}", received);
            self.connection.buffer.clear();
            match received {
                Ok(maby_message) => {
                    if let Some(m) = maby_message {
                        match &m.command {
                            Command::Pass{password} => {
                                match self.connection.state.registration_state {
                                    RegistrationState::None => {
                                        if password == "blabla" { // Match connection password
                                            self.connection.state.registration_state = RegistrationState::PassReceived;
                                        } else {
                                            warn!("{:?}", IRCError::PasswdMismatch);
                                        }
                                    },
                                    _ => {
                                        warn!("{:?}", IRCError::AlreadyRegistred);
                                    }
                                }
                            },
                            Command::Nick{nickname} => {
                                match self.connection.state.registration_state {
                                    RegistrationState::PassReceived => {
                                        // TODO: check if in use
                                        self.connection.state.registration_state = RegistrationState::NickReceived;
                                        self.connection.state.nick = nickname.to_string();
                                    },
                                    _ => !todo!()
                                }
                            },
                            _ => todo!()

                        }
                        let mut msg = m.to_string();
                        msg.push('\n');
                        self.connection.stream.write_all(msg.as_bytes()).await.unwrap();

                        let mut db = self.db.lock().await;
                        db.push(msg);
                        drop(db);
                    }
                },
                Err(err) => {
                    match err {
                        IRCError::ClientExited => {
                            return Err(());
                        },
                        _ => {
                            warn!("{:?}", err);
                        },
                    }
                },
            }
        }
        Ok(())
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
        db: Arc::new(tokio::sync::Mutex::new(Vec::new())),
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

