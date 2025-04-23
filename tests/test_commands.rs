use irc_proto::{enable_logging, types::{Command, Message}};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream, time::sleep};
use log::info;
use std::time::{Duration, Instant};
use serial_test::serial;

use irc_server::server::start_server;


async fn register(stream: &mut TcpStream, nickname: String) {
    stream.write_all(
        Message{
            tags: None,
            source: None,
            command: Command::PASS { password: "password".to_string() }
        }.to_bytes().as_bytes()
    ).await.unwrap();

    stream.write_all(
        Message{
            tags: None,
            source: None,
            command: Command::NICK { nickname: nickname.clone() }
        }.to_bytes().as_bytes()
    ).await.unwrap();

    stream.write_all(
        Message{
            tags: None,
            source: None,
            command: Command::USER {
                user: nickname.clone(),
                mode: "0".to_string(),
                unused: "*".to_string(),
                realname: nickname.clone(),
            }
        }.to_bytes().as_bytes()
    ).await.unwrap();
}

#[serial]
#[tokio::test]
async fn test_ping() {
    enable_logging();
    let addr = start_server().await;
    let mut stream = TcpStream::connect(addr).await.unwrap();

    let now = Instant::now();

    let message = Message { tags: None, source: None, command: Command::PING { token: "token".to_string() } };
    stream.write_all(message.to_bytes().as_bytes()).await.unwrap();
    let mut response = [0; 21];
    stream.read_exact(&mut response).await.unwrap();

    let elapsed = now.elapsed();
    info!("Elapsed: {:.4?}", elapsed);

    assert_eq!(
        ":server1 PONG token\r\n".as_bytes(), response,
    );
}

#[serial]
#[tokio::test]
async fn test_ping_multiple() {
    let addr = start_server().await;
    let mut stream = TcpStream::connect(addr).await.unwrap();

    stream.write_all(b"PING token1\r\nPING token2\r\n").await.unwrap();
    let mut response = [0; 22];
    stream.read_exact(&mut response).await.unwrap();
    assert_eq!(
        ":server1 PONG token1\r\n".as_bytes(),
        response
    );

    let mut response = [0; 22];
    stream.read_exact(&mut response).await.unwrap();
    assert_eq!(
        ":server1 PONG token2\r\n".as_bytes(),
        response
    );
}

#[serial]
#[tokio::test]
async fn test_invalid_message() {
    let addr = start_server().await;
    let mut stream = TcpStream::connect(addr).await.unwrap();

    stream.write_all(b"PING token1\r\nINVALID\r\nPING token2\r\n").await.unwrap();
    let mut response = [0; 22];
    stream.read_exact(&mut response).await.unwrap();
    assert_eq!(
        ":server1 PONG token1\r\n".as_bytes(),
        response
    );

    let mut response = [0; 22];
    stream.read_exact(&mut response).await.unwrap();
    assert_eq!(
        ":server1 PONG token2\r\n".as_bytes(),
        response
    );
}

#[serial]
#[tokio::test]
async fn test_partial() {
    let addr = start_server().await;
    let mut stream = TcpStream::connect(addr).await.unwrap();

    stream.write_all(b"PING ").await.unwrap();
    sleep(Duration::from_secs(1)).await;
    stream.write_all(b"token1\r\n").await.unwrap();
    let mut response = [0; 22];
    stream.read_exact(&mut response).await.unwrap();
    assert_eq!(
        ":server1 PONG token1\r\n".as_bytes(),
        response
    );
}

#[serial]
#[tokio::test]
async fn test_message() {
    let server_addr = start_server().await;

    let mut client1 = TcpStream::connect(server_addr).await.unwrap();
    register(&mut client1, "nick1".to_string()).await;

    let mut client2 = TcpStream::connect(server_addr).await.unwrap();
    register(&mut client2, "nick2".to_string()).await;

    tokio::time::sleep(tokio::time::Duration::from_micros(1)).await;

    client1.write_all(
        Message{
            tags: None,
            source: None,
            command: Command::PRIVMSG {
                targets: "nick2".to_string(),
                text: "hello".to_string(),
            }
        }.to_bytes().as_bytes()
    ).await.unwrap();

    let mut response = [0; 28];
    client2.read(&mut response).await.unwrap();
    assert_eq!(
        ":nick1 PRIVMSG nick2 hello\r\n".as_bytes(),
        &response
    );
}

// #[tokio::test]
// async fn test_channel() {
//     init_logger();
//     let server_addr = start_server().await;
//     let now = Instant::now();
//
//     let mut client1 = TcpStream::connect(server_addr).await.unwrap();
//     register(&mut client1, "nick1".to_string()).await;
//
//     let mut client2 = TcpStream::connect(server_addr).await.unwrap();
//     register(&mut client2, "nick2".to_string()).await;
//
//     // write_message(&mut client1, Message{
//     //     prefix: None,
//     //     content: Content::Command(Command::JOIN{
//     //         channels: "#channel1".to_string(),
//     //         keys: None,
//     //     }),
//     // }).await;
//     let mut response = [0; 23];
//     client1.read(&mut response).await.unwrap();
//     assert_eq!(
//         ":nick1 JOIN #channel1\r\n".as_bytes(),
//         &response
//     );
//
//     // write_message(&mut client2, Message{
//     //     prefix: None,
//     //     content: Content::Command(Command::JOIN{
//     //         channels: "#channel1".to_string(),
//     //         keys: None,
//     //     }),
//     // }).await;
//     let mut response = [0; 23];
//     client2.read(&mut response).await.unwrap();
//     assert_eq!(
//         ":nick2 JOIN #channel1\r\n".as_bytes(),
//         &response
//     );
//
//     // write_message(&mut client1, Message{
//     //     prefix: None,
//     //     content: Content::Command(Command::PRIVMSG{
//     //         targets: "#channel1".to_string(),
//     //         text: "11111111".to_string(),
//     //     }),
//     // }).await;
//     let mut response = [0; 35];
//     client2.read(&mut response).await.unwrap();
//     assert_eq!(
//         ":nick1 PRIVMSG #channel1 11111111\r\n".as_bytes(),
//         &response
//     );
//     let elapsed = now.elapsed();
//     info!("Elapsed: {:.4?}", elapsed);
// }
