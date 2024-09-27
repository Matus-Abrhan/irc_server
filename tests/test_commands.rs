use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};

use irc_server::server::start_server;

#[tokio::test]
async fn test_pass() {
    start_server().await;

    let mut stream = TcpStream::connect("127.0.0.1:1234").await.unwrap();
    stream.write_all(b":prefix PASS passwd abc def a b c d e f g\n").await.unwrap();

    let mut response = [0; 149];
    stream.read_exact(&mut response).await.unwrap();
    assert_eq!(
        "Message { prefix: Some(\":prefix\"), command: Pass { password: \"passwd\" } }".as_bytes(),
        &response
    )

}
