use tokio::net::{TcpListener, TcpStream};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use log::info;

// mod core;
// use core::connection::Connection;
use irc_server::irc_core::connection::Connection;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    loop {
        let (stream, addr): (TcpStream, SocketAddr) = listener.accept().await.unwrap();

        tokio::spawn(async move {
            process(stream, addr).await;
        });
    }
}


async fn process(stream: TcpStream, address: SocketAddr) {

    let mut conn: Connection = Connection::new(stream, address);
    info!("{:} connected", conn.address);

    loop {
        match conn.read_message().await {
            Ok(m) => {
                info!("{:?}", m)
            },
            Err(_e) => {
                info!("{:} exited", conn.address);
                return;
            },
        };
    }
}
