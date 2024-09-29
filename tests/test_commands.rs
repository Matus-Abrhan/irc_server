use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};

use irc_server::server::start_server;

#[tokio::test]
async fn test_pass() {
    let addr = start_server().await;

    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream.write_all(b":prefix PASS passwd abc def a b c d e f g\n").await.unwrap();

    let mut response = [0; 73];
    stream.read_exact(&mut response).await.unwrap();
    assert_eq!(
        "Message { prefix: Some(\":prefix\"), command: Pass { password: \"passwd\" } }".as_bytes(),
        &response
    );
}
