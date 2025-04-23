use irc_proto::enable_logging;
use tokio::net::TcpListener;
use std::net::SocketAddr;
use log::info;

use irc_server::server::run;
use irc_server::config::CONFIG;

#[tokio::main]
async fn main() -> Result<(), ()> {
    enable_logging();

    let config = CONFIG.lock().unwrap();
    let server_addr: SocketAddr = SocketAddr::new(config.server.address_v4, config.server.port);
    drop(config);

    let listener = match TcpListener::bind(server_addr).await {
        Ok(l) => l,
        Err(e) => panic!("{}", e),
    };
    info!("Server started at {:}", server_addr);
    run(listener, tokio::signal::ctrl_c()).await
}
