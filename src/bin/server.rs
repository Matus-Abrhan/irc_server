use tokio::net::TcpListener;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use log::info;

use irc_server::server::run;

#[tokio::main]
async fn main() -> Result<(), ()> {
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
    run(listener, tokio::signal::ctrl_c()).await
}


