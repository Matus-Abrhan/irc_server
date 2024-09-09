use log::info;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

mod core;
use core::message::Message;

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
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf = [0; 1024];

            loop {

                let n = match socket.read(&mut buf).await {
                    Ok(n) if n == 0 => return,
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("failed to read from socket; err = {:?}", e);
                        return;
                    }
                };

                // Write the data back
                let buf_str = std::str::from_utf8(&buf).unwrap().split_once("\n").unwrap().0;
                let message = Message::try_from(buf_str);
                info!("{:?}", message);
                if let Err(e) = socket.write_all(&buf[0..n]).await {
                    eprintln!("failed to write to socket; err = {:?}", e);
                    return;
                }
            }
        });
    }
}
