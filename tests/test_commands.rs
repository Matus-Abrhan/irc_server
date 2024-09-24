use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use log::info;

use irc_server::irc_core::connection::Connection;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::init();

    let server_addr: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 1234);

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
        let message = match conn.read_message().await {
            Ok(m) => m,
            Err(_e) => {
                info!("{:} exited", conn.address);
                return;
            },
        };
        if let Some(m) = message {
            conn.stream.write_all(m.to_string().as_bytes()).await.unwrap();
        }
    }
}

#[tokio::test]
async fn test_pass() {
    tokio::spawn(async move {
        let _ = main();
    });

    let mut stream = TcpStream::connect("127.0.0.1:1234").await.unwrap();
    stream.write_all(b":prefix PASS passwd abc def a b c d e f g\n").await.unwrap();

    let mut response = [0; 149];
    stream.read_exact(&mut response).await.unwrap();
    assert_eq!(
        "Message { prefix: Some(\":prefix\"), command: Pass { password: \"passwd\", version: \"abc\", flags: \"def\", options: [\"a\", \"b\", \"c\", \"d\", \"e\", \"f\", \"g\"] } }".as_bytes(),
        &response
    )

}
