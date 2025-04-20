use once_cell::sync::Lazy;
use tokio::net::TcpListener;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use log::info;

use irc_server::server::run;
use irc_server::config::{Config, CONFIG};

// pub stati CONFIG: Lazy<Arc<Mutex<Config>>> = Lazy::new(|| {
//     Arc::new(Mutex::new(Config::new("config.toml")))
// });

#[tokio::main]
async fn main() -> Result<(), ()> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::init();

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
