use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

pub struct App {
    listener: TcpListener,
}

impl App {
    pub async fn new(addr: String) -> Self {
        let listener: TcpListener =
            TcpListener::bind(addr).await.expect("Failed to bind to address");
        App { listener }
    }

    pub async fn run(self) {
        while let Ok((stream, _addr)) = self.listener.accept().await {
            tokio::spawn(async move {
                Self::server_loop(stream).await;
            });
        }
    }
    async fn server_loop(mut stream: TcpStream) {
        let mut buf = [0; 1024];
        loop {
            match stream.read(&mut buf).await {
                Ok(n) if n > 0 => {
                    println!("got request");
                    if stream.write_all(b"+OK\r\n").await.is_err() {
                        println!("Failed to write response, dropping connection.");
                        break;
                    }
                }
                Ok(_) => {
                    println!("Klient się rozłączył");
                    break;
                }
                Err(e) => {
                    println!("Connection error: {}", e);
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;
    #[tokio::test]
    async fn test_health_check() {
        let app = App::new("127.0.0.1:0".to_string()).await;
        let addr = app.listener.local_addr().unwrap();

        tokio::spawn(async move {
            app.run().await;
        });
        let mut stream = TcpStream::connect(addr).await.unwrap();
        let request_data = "GET /health HTTP/1.1\r\n\r\n";
        stream.write_all(request_data.as_bytes()).await.unwrap();
        let mut buf = [0; 1024];
        let n = stream.read(&mut buf).await.unwrap();
        let response = String::from_utf8_lossy(&buf[..n]);

        assert!(response.contains("+OK"));
    }
}
