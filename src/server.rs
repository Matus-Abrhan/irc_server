use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use log::{info, warn};
use std::collections::HashMap;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio::time::{self, Duration};

use irc_proto::types::Message;
use irc_proto::connection::Connection;

use crate::handler::Handler;

const BACKOFF_LIMIT: u64 = 64;
pub type MessageReceiverMap = Arc<Mutex<HashMap<String, mpsc::Sender<Message>>>>;

struct Listener {
    listener: TcpListener,
    handler_tx_map: MessageReceiverMap,
    notify_shutdown: broadcast::Sender<()>,
}

impl Listener {
    async fn run(&self) -> Result<(), ()> {
        loop {
            let (stream, address) = self.accept().await?;

            // TODO: move to user registration
            let (handler_tx, handler_rx) = mpsc::channel(1);
            let mut receiver_map = self.handler_tx_map.lock().await;
            receiver_map.insert("".to_string(), handler_tx);
            drop(receiver_map);

            let mut handler = Handler::new(
                Connection::new(stream, address),
                handler_rx,
                self.notify_shutdown.subscribe(),
            );
            info!("{:} connected", handler.connection.address());

            tokio::spawn(async move {
                if (handler.run().await).is_err() {
                    info!("{:} exited", handler.connection.address());
                }
            });
        }
    }

    async fn accept(&self) -> Result<(TcpStream, SocketAddr), ()> {
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


pub async fn run(listener: TcpListener, shutdown: impl Future) -> Result<(), ()> {
    let (notify_shutdown, _) = broadcast::channel(1);

    // let (server_tx, server_rx) = mpsc::channel(1);
    let server = Listener {
        listener,
        handler_tx_map: Arc::new(Mutex::new(HashMap::new())),
        notify_shutdown,
    };
    tokio::select! {
        _ = server.run() => {}
        _ = shutdown => {}
    }

    let Listener{
        notify_shutdown,
        ..
    } = server;
    drop(notify_shutdown);

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

    return server_addr;
}

