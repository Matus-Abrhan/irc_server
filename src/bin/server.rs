use tokio::net::{TcpListener, TcpStream};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use log::info;

use irc_server::server::run;

#[tokio::main]
async fn main() -> Result<(), ()> {
    // if std::env::var_os("RUST_LOG").is_none() {
    //     std::env::set_var("RUST_LOG", "debug");
    // }
    // env_logger::init();
    //
    // let server_addr: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1234);
    //
    // let listener = match TcpListener::bind(server_addr).await {
    //     Ok(l) => l,
    //     Err(e) => panic!("{}", e),
    // };
    // info!("Server started at {:}", server_addr);
    //
    // loop {
    //     let (stream, addr): (TcpStream, SocketAddr) = listener.accept().await.unwrap();
    //
    //     tokio::spawn(async move {
    //         process(stream, addr).await;
    //     });
    // }
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
    run(listener).await
}


// async fn process(stream: TcpStream, address: SocketAddr) {
//
//     let mut conn: Connection = Connection::new(stream, address);
//     info!("{:} connected", conn.address);
//
//     loop {
//         let message = match conn.read_message().await {
//             Ok(m) => m,
//             Err(_e) => {
//                 info!("{:} exited", conn.address);
//                 return;
//             },
//         };
//         if let Some(m) = message {
//             conn.stream.write_all(m.to_string().as_bytes()).await.unwrap();
//         }
//     }
// }
