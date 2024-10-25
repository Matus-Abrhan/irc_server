use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};

use irc_server::server::start_server;

#[tokio::test]
async fn test_ping() {
    let addr = start_server().await;

    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream.write_all(b"PING token\r\n").await.unwrap();

    let mut response = [0; 12];
    stream.read_exact(&mut response).await.unwrap();
    assert_eq!(
        "PONG token\r\n".as_bytes(),
        &response
    );
}

#[tokio::test]
async fn test_ping_multiple() {
    let addr = start_server().await;

    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream.write_all(b"PING token1\r\nPING token2\r\n").await.unwrap();

    let mut response = [0; 13];
    stream.read_exact(&mut response).await.unwrap();
    assert_eq!(
        "PONG token1\r\n".as_bytes(),
        &response
    );

    let mut response = [0; 13];
    stream.read_exact(&mut response).await.unwrap();
    assert_eq!(
        "PONG token2\r\n".as_bytes(),
        &response
    );
}

#[tokio::test]
async fn test_invalid_message() {
    let addr = start_server().await;

    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream.write_all(b"PING token1\r\nINVALID\r\nPING token2\r\n").await.unwrap();

    let mut response = [0; 13];
    stream.read_exact(&mut response).await.unwrap();
    assert_eq!(
        "PONG token1\r\n".as_bytes(),
        &response
    );

    let mut response = [0; 13];
    stream.read_exact(&mut response).await.unwrap();
    assert_eq!(
        "PONG token2\r\n".as_bytes(),
        &response
    );
}
