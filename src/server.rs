use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use log::{info, warn};
use std::collections::HashMap;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio::time::{self, Duration};

use irc_proto::message::Message;

use crate::connection::{Connection, ConnectionTxMap, ChannelMap};
use crate::config::{Config, Server};

const BACKOFF_LIMIT: u64 = 64;

struct Listener {
    listener: TcpListener,
    connection_tx_map: ConnectionTxMap,
    channel_map: ChannelMap,
    config: Config,
    notify_shutdown: broadcast::Sender<()>,
    shutdown_complete_tx: mpsc::Sender<()>,
    shutdown_complete_rx: mpsc::Receiver<()>,
}

struct Handler {
    connection: Connection,
    connection_rx: mpsc::Receiver<Message>,
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
                connection_rx,
                connection: Connection::new(
                    stream,
                    address,
                    self.config.clone(),
                    self.connection_tx_map.clone(),
                    self.channel_map.clone(),
                ),
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
            tokio::select! {
                client_result = self.connection.read_message() => {
                    self.connection.process_client_result(client_result).await?;
                },

                server_result_option = self.connection_rx.recv() => {
                    if let Some(server_result) = server_result_option {
                        self.connection.process_server_result(server_result).await;
                    }
                },

                _ = self.shutdown.recv() => {
                    info!("Server quit");
                    return Err(());
                },
            };
        }
        return Ok(());
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

pub async fn run(listener: TcpListener, config: Config, shutdown: impl Future) -> Result<(), ()> {
    let (notify_shutdown, _) = broadcast::channel(1);
    let (shutdown_complete_tx, shutdown_complete_rx) = mpsc::channel(1);

    let mut server = Listener {
        listener,
        connection_tx_map: Arc::new(Mutex::new(HashMap::new())),
        channel_map: Arc::new(Mutex::new(HashMap::new())),
        config,
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
    let config = Config{server: Server{
        name: "server1".to_string(), password: "password".to_string(),
        port: 0, address_v4: "127.0.0.1".to_string()}
    };
    let listener = match TcpListener::bind("127.0.0.1:0").await {
        Ok(l) => l,
        Err(e) => panic!("{}", e),
    };
    let server_addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        run(listener, config, tokio::signal::ctrl_c()).await
    });

    server_addr
}

