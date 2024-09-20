use tokio::{io::AsyncReadExt, net::{TcpListener, TcpStream}};
use std::{io::Cursor, net::SocketAddr};
use log::info;

mod core;
use core::{command::Command, connection::Connection, message::{Error, Message}};
use core::message::get_message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::init();

    let listener = match TcpListener::bind("127.0.0.1:1234").await {
        Ok(l) => l,
        Err(e) => panic!("{}", e),
    };
    info!("Server started");

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

    // while let Ok(res) = conn.read_message().await {
    //     info!("{:?}", res);
    // }
    // info!("{:} exited", conn.address);

    loop {
        match conn.read_message().await {
            Ok(m) => info!("{:?}", m),
            Err(_e) => {
                info!("{:} exited", conn.address);
                return;
            },
        };
    }


    // loop {
    //     let _n = match conn.stream.read_buf(&mut conn.buffer).await {
    //         Ok(0) => { info!("{:} exited", conn.address);
    //             return;
    //         },
    //         Ok(n) => n,
    //         Err(e) => {
    //             eprintln!("failed to read from socket; err = {:?}", e);
    //             return;
    //         }
    //     };
    //
    //     let mut cursor = Cursor::new(&conn.buffer[..]);
    //     cursor.set_position(0);
    //     match Message::parse(&mut cursor) {
    //         Ok(m) => info!("{:?}", m),
    //         Err(_e) => (),
    //     };
    //     info!("{:?}", &conn.buffer[..]);
    //     conn.buffer.clear();
    // }
}
