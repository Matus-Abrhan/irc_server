use tokio::net::TcpListener;
use std::net::{IpAddr, SocketAddr};
use log::info;

use irc_server::server::run;
use irc_server::config::Config;

#[tokio::main]
async fn main() -> Result<(), ()> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::init();

    let config = Config::read("config.toml");

    // let server_addr: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6697);
    let server_addr: SocketAddr = SocketAddr::new(IpAddr::V4(config.server.address_v4.parse().expect("IP parse failed")), config.server.port);

    let listener = match TcpListener::bind(server_addr).await {
        Ok(l) => l,
        Err(e) => panic!("{}", e),
    };
    info!("Server started at {:}", server_addr);
    run(listener, config, tokio::signal::ctrl_c()).await
}


