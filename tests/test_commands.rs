use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};
use log::info;

use irc_server::server::start_server;
use irc_server::irc_core::message::{Message, Content};
use irc_server::irc_core::command::Command;

fn init_logger() {
    // NOTE: also use "RUST_LOG=debug cargo test"
    let _ = env_logger::builder().is_test(true).try_init();
}

async fn write_message(stream: &mut TcpStream, message: &Message) {
    let mut msg_parts = message.deserialize().join(" ");
    msg_parts.push_str("\r\n");
    let bytes = msg_parts.as_bytes();
    info!("messages parts: {}", msg_parts);

    stream.write_all(bytes).await.unwrap();
    stream.flush().await.unwrap();
}

async fn register(stream: &mut TcpStream, nickname: String) {
    write_message(stream, &Message{
        prefix: None,
        content: Content::Command(Command::Pass{
            password: "blabla".to_string()
        }),
    }).await;

    write_message(stream, &Message{
        prefix: None,
        content: Content::Command(Command::Nick{nickname: nickname.clone()}),
    }).await;

    write_message(stream, &Message{
        prefix: None,
        content: Content::Command(Command::User{
            user: nickname.clone(),
            mode: "0".to_string(),
            unused: "*".to_string(),
            realname: nickname.clone(),
        }),
    }).await;
}

#[tokio::test]
async fn test_ping() {
    let addr = start_server().await;

    let mut stream = TcpStream::connect(addr).await.unwrap();

    let mut msg_parts = Message{prefix: None, content: Content::Command(Command::Ping{token: "token".to_string()})}.deserialize().join(" ");
    msg_parts.push_str("\r\n");
    let bytes = msg_parts.as_bytes();
    stream.write_all(bytes).await.unwrap();

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

#[tokio::test]
async fn test_message() {
    let server_addr = start_server().await;

    let mut client1 = TcpStream::connect(server_addr).await.unwrap();
    register(&mut client1, "nick1".to_string()).await;

    let mut client2 = TcpStream::connect(server_addr).await.unwrap();
    register(&mut client2, "nick2".to_string()).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    write_message(&mut client1, &Message{
        prefix: None,
        content: Content::Command(Command::PrivMsg{
            targets: "nick2".to_string(),
            text: "11111111".to_string(),
        }),
    }).await;

    let mut response = [0; 31];
    client2.read(&mut response).await.unwrap();
    assert_eq!(
        ":nick1 PRIVMSG nick2 11111111\r\n".as_bytes(),
        &response
    );
}

#[tokio::test]
async fn test_channel() {
    let server_addr = start_server().await;

    let mut client1 = TcpStream::connect(server_addr).await.unwrap();
    register(&mut client1, "nick1".to_string()).await;

    let mut client2 = TcpStream::connect(server_addr).await.unwrap();
    register(&mut client2, "nick2".to_string()).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    write_message(&mut client1, &Message{
        prefix: None,
        content: Content::Command(Command::Join{
            channels: "#channel1".to_string(),
            keys: None,
        }),
    }).await;
    let mut response = [0; 23];
    client1.read(&mut response).await.unwrap();
    assert_eq!(
        ":nick1 JOIN #channel1\r\n".as_bytes(),
        &response
    );

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    write_message(&mut client2, &Message{
        prefix: None,
        content: Content::Command(Command::Join{
            channels: "#channel1".to_string(),
            keys: None,
        }),
    }).await;
    let mut response = [0; 23];
    client2.read(&mut response).await.unwrap();
    assert_eq!(
        ":nick2 JOIN #channel1\r\n".as_bytes(),
        &response
    );


    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    write_message(&mut client1, &Message{
        prefix: None,
        content: Content::Command(Command::PrivMsg{
            targets: "#channel1".to_string(),
            text: "11111111".to_string(),
        }),
    }).await;
    let mut response = [0; 31];
    client2.read(&mut response).await.unwrap();
    assert_eq!(
        ":nick1 PRIVMSG nick2 11111111\r\n".as_bytes(),
        &response
    );


}
