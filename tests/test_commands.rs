use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};

use irc_server::server::start_server;

#[tokio::test]
async fn test_ping() {
    let addr = start_server().await;

    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream.write_all(b":prefix PING token\n").await.unwrap();

    let mut response = [0; 83];
    stream.read_exact(&mut response).await.unwrap();
    assert_eq!(
        "Message { prefix: Some(\":Server\"), command: Pong { server: None, token: \"token\" } }".as_bytes(),
        &response
    );
}

#[tokio::test]
async fn test_register() {

}
