use tokio::net::{TcpListener, TcpStream};
use std::net::SocketAddr;
use log::info;

mod core;
use core::connection::Connection;

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
        let (stream, _addr): (TcpStream, SocketAddr) = listener.accept().await.unwrap();

        tokio::spawn(async move {
            process(stream).await;
        });
    }
}


async fn process(stream: TcpStream) {

    let mut conn: Connection = Connection::new(stream);
    loop {
        let message = match conn.read_message().await {
            Ok(m) => m, 
            Err(_) => None,
        };
        info!("{:?}", message);
    }


    // loop {
    //     let n = match conn.stream.read_buf(&mut conn.buffer).await {
    //         Ok(n) if n == 0 => return,
    //         Ok(n) => n,
    //         Err(e) => {
    //             eprintln!("failed to read from socket; err = {:?}", e);
    //             return;
    //         }
    //     };
    //
    //     // Write the data back
    //     if let Err(e) = conn.stream.write_all(&conn.buffer[0..n]).await {
    //         eprintln!("failed to write to socket; err = {:?}", e);
    //         return;
    //     }
    //     conn.buffer.clear();
    // }
}
