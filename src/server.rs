use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use log::{error, info, warn};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{self, Duration};

use crate::irc_core::connection::Connection;

type Db = Arc<Mutex<HashMap<String, String>>>;

const BACKOFF_LIMIT: u64 = 64;

struct Listener {
    db: Db,
    listener: TcpListener,
}

struct Handler {
    db: Db,
    connection: Connection
}

impl Listener {
    async fn run(&mut self) -> Result<(), ()> {

        loop {
            let (stream, address) = self.accept().await?;
            let mut handler = Handler{db: self.db.clone(), connection: Connection::new(stream, address)};
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
        loop {
            let message = match self.connection.read_message().await {
                Ok(m) => m,
                Err(_e) => {
                    return Err(());
                },
            };
            if let Some(m) = message {
                self.connection.stream.write_all(m.to_string().as_bytes()).await.unwrap();
            }
        }
    }
}

pub async fn run(listener: TcpListener) -> Result<(), ()> {
    let mut server = Listener { listener, db: Arc::new(Mutex::new(HashMap::new()))};
    server.run().await?;
    Ok(())
}

pub async fn start_server() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::init();

    let server_addr: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1234);

    let listener = match TcpListener::bind(server_addr).await {
        Ok(l) => l,
        Err(e) => panic!("{}", e),
    };
    info!("Server started at {:}", server_addr);
    tokio::spawn(async move {
        run(listener).await
    });
}


