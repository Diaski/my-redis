use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

async fn spawn_test_server() -> std::net::SocketAddr {
    let app = my_redis::server::App::new("127.0.0.1:0".to_string()).await;
    let addr = app.listener.local_addr().unwrap();
    tokio::spawn(async move {
        app.run().await;
    });
    addr
}

async fn send_resp(addr: std::net::SocketAddr, raw: &str) -> String {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream.write_all(raw.as_bytes()).await.unwrap();
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).await.unwrap();
    String::from_utf8_lossy(&buf[..n]).to_string()
}

#[tokio::test]
async fn test_ping_returns_pong() {
    let addr = spawn_test_server().await;
    let resp = send_resp(addr, "*1\r\n$4\r\nPING\r\n").await;
    assert_eq!(resp, "+PONG\r\n");
}

#[tokio::test]
async fn test_ping_case_insensitive() {
    let addr = spawn_test_server().await;
    let resp = send_resp(addr, "*1\r\n$4\r\nping\r\n").await;
    assert_eq!(resp, "+PONG\r\n");
}

#[tokio::test]
async fn test_set_returns_ok() {
    let addr = spawn_test_server().await;
    let resp = send_resp(addr, "*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n").await;
    assert_eq!(resp, "+OK\r\n");
}

#[tokio::test]
async fn test_get_existing_key() {
    let addr = spawn_test_server().await;
    let mut stream = TcpStream::connect(addr).await.unwrap();

    stream.write_all(b"*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n").await.unwrap();
    let mut buf = [0u8; 64];
    stream.read(&mut buf).await.unwrap();

    stream.write_all(b"*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n").await.unwrap();
    let n = stream.read(&mut buf).await.unwrap();
    let resp = String::from_utf8_lossy(&buf[..n]);

    assert_eq!(resp, "$3\r\nbar\r\n");
}

#[tokio::test]
async fn test_get_missing_key_returns_null() {
    let addr = spawn_test_server().await;
    let resp = send_resp(addr, "*2\r\n$3\r\nGET\r\n$11\r\nnonexistent\r\n").await;
    assert_eq!(resp, "$-1\r\n");
}

#[tokio::test]
async fn test_set_overwrite_key() {
    let addr = spawn_test_server().await;
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let mut buf = [0u8; 64];

    stream.write_all(b"*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n").await.unwrap();
    stream.read(&mut buf).await.unwrap();

    stream.write_all(b"*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbaz\r\n").await.unwrap();
    stream.read(&mut buf).await.unwrap();

    stream.write_all(b"*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n").await.unwrap();
    let n = stream.read(&mut buf).await.unwrap();
    let resp = String::from_utf8_lossy(&buf[..n]);

    assert_eq!(resp, "$3\r\nbaz\r\n");
}

#[tokio::test]
async fn test_del_existing_key() {
    let addr = spawn_test_server().await;
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let mut buf = [0u8; 64];

    stream.write_all(b"*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n").await.unwrap();
    stream.read(&mut buf).await.unwrap();

    stream.write_all(b"*2\r\n$3\r\nDEL\r\n$3\r\nfoo\r\n").await.unwrap();
    let n = stream.read(&mut buf).await.unwrap();
    let resp = String::from_utf8_lossy(&buf[..n]);
    assert_eq!(resp, ":1\r\n");
}

#[tokio::test]
async fn test_del_missing_key_returns_zero() {
    let addr = spawn_test_server().await;
    let resp = send_resp(addr, "*2\r\n$3\r\nDEL\r\n$7\r\nmissing\r\n").await;
    assert_eq!(resp, ":0\r\n");
}

#[tokio::test]
async fn test_exists_returns_one_when_key_present() {
    let addr = spawn_test_server().await;
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let mut buf = [0u8; 64];

    stream.write_all(b"*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n").await.unwrap();
    stream.read(&mut buf).await.unwrap();

    stream.write_all(b"*2\r\n$6\r\nEXISTS\r\n$3\r\nfoo\r\n").await.unwrap();
    let n = stream.read(&mut buf).await.unwrap();
    let resp = String::from_utf8_lossy(&buf[..n]);
    assert_eq!(resp, ":1\r\n");
}

#[tokio::test]
async fn test_exists_returns_zero_when_missing() {
    let addr = spawn_test_server().await;
    let resp = send_resp(addr, "*2\r\n$6\r\nEXISTS\r\n$7\r\nmissing\r\n").await;
    assert_eq!(resp, ":0\r\n");
}

#[tokio::test]
async fn test_unknown_command_returns_error() {
    let addr = spawn_test_server().await;
    let resp = send_resp(addr, "*1\r\n$6\r\nFOOBAR\r\n").await;
    assert!(resp.starts_with('-'));
}

#[tokio::test]
async fn test_set_requires_two_args() {
    let addr = spawn_test_server().await;
    let resp = send_resp(addr, "*2\r\n$3\r\nSET\r\n$3\r\nfoo\r\n").await;
    assert!(resp.starts_with('-'));
}

#[tokio::test]
async fn test_multiple_keys_are_independent() {
    let addr = spawn_test_server().await;
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let mut buf = [0u8; 64];

    stream.write_all(b"*3\r\n$3\r\nSET\r\n$1\r\na\r\n$1\r\n1\r\n").await.unwrap();
    stream.read(&mut buf).await.unwrap();

    stream.write_all(b"*3\r\n$3\r\nSET\r\n$1\r\nb\r\n$1\r\n2\r\n").await.unwrap();
    stream.read(&mut buf).await.unwrap();

    stream.write_all(b"*2\r\n$3\r\nGET\r\n$1\r\na\r\n").await.unwrap();
    let n = stream.read(&mut buf).await.unwrap();
    assert_eq!(String::from_utf8_lossy(&buf[..n]), "$1\r\n1\r\n");

    stream.write_all(b"*2\r\n$3\r\nGET\r\n$1\r\nb\r\n").await.unwrap();
    let n = stream.read(&mut buf).await.unwrap();
    assert_eq!(String::from_utf8_lossy(&buf[..n]), "$1\r\n2\r\n");
}
