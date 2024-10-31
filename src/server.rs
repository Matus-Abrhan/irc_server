use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use log::{info, warn};
use std::collections::HashMap;

// use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio::time::{self, Duration};

use crate::irc_core::connection::Connection;
use crate::irc_core::message::Message;
// use crate::irc_core::channel::Channel;
use crate::irc_core::error::IRCError;

// type ChannelDb = Arc<Mutex<Vec<Channel>>>;
// type ConnectionDb = Arc<Mutex<Vec<State>>>;

const BACKOFF_LIMIT: u64 = 64;

struct Listener {
    listener: TcpListener,
    // channel_db: ChannelDb,
    // connection_db: ConnectionDb,
    connection_tx_map: Arc<Mutex<HashMap<String, mpsc::Sender<Result<Option<Message>, IRCError>>>>>,
    notify_shutdown: broadcast::Sender<()>,
    shutdown_complete_tx: mpsc::Sender<()>,
    shutdown_complete_rx: mpsc::Receiver<()>,
}

struct Handler {
    connection: Connection,
    // channel_db: ChannelDb,
    // connection_db: ConnectionDb,
    task_receiver: mpsc::Receiver<Result<Option<Message>, IRCError>>,
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
            let (connection_tx, connection_rx) = mpsc::channel(1);
            let mut connection_tx_map = self.connection_tx_map.lock().await;
            connection_tx_map.insert(address.to_string(), connection_tx);
            drop(connection_tx_map);
            let mut handler = Handler{
                // channel_db: self.channel_db.clone(),
                // connection_db: self.connection_db.clone(),
                task_receiver: connection_rx,
                connection: Connection::new(stream, address, self.connection_tx_map.clone()),
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
                res = self.task_receiver.recv() => {
                    match res {
                        Some(res) => {
                            match res {
                                Ok(Some(res)) => {
                                    self.connection.write_message(&res).await;
                                    Ok(None)
                                },
                                _ => Ok(None),
                            }
                        },
                        None => Ok(None),
                    }
                },
                _ = self.shutdown.recv() => {
                    info!("Server quit");
                    return Err(());
                }
            };

            match received {
                Ok(maby_message) => {
                    match maby_message {
                        Some(message) => {
                            self.connection.write_response(message).await
                        },
                        None => {
                            (); // NOTE: No message received no response sent
                        },
                    }
                },
                Err(err) => {
                    match err {
                        IRCError::ClientExited => {
                            // TODO: remove from map on QUIT message?
                            let mut connection_tx_map = self.connection.connection_tx_map.lock().await;
                            if connection_tx_map.remove(&self.connection.address.to_string()).is_none() {
                                if connection_tx_map.remove(&self.connection.state.nickname).is_none() {
                                    warn!("This should not happen");
                                }
                            }
                            drop(connection_tx_map);

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
        // channel_db: Arc::new(Mutex::new(Vec::new())),
        // connection_db: Arc::new(Mutex::new(Vec::new())),
        connection_tx_map: Arc::new(Mutex::new(HashMap::new())),
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

