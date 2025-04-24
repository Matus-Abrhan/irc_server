use std::future::Future;
use std::net::SocketAddr;
use log::{info, warn};

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc};
use tokio::time::{self, Duration};

use irc_proto::connection::Connection;

use crate::bridge::{Bridge, CommMsg, OperMsg};
use crate::handler::Handler;

const BACKOFF_LIMIT: u64 = 64;

struct Listener {
    listener: TcpListener,
    notify_shutdown: broadcast::Sender<()>,
}

impl Listener {
    async fn run(&self, oper_tx: mpsc::Sender<OperMsg>, comm_tx: mpsc::Sender<CommMsg>) -> Result<(), ()> {
        loop {
            let (stream, address) = self.accept().await?;

            let mut handler = Handler::new(
                Connection::new(stream, address),
                oper_tx.clone(),
                comm_tx.clone(),
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
    let (oper_tx, oper_rx) = mpsc::channel(1);
    let (comm_tx, comm_rx) = mpsc::channel(1);

    let server = Listener {
        listener,
        notify_shutdown,
    };
    let mut bridge = Bridge::new(oper_rx, comm_rx);

    tokio::spawn(async move {
        if (bridge.run().await).is_err() {
        }
    });

    tokio::select! {
        _ = server.run(oper_tx, comm_tx) => {}
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

