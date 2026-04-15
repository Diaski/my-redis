use std::net::TcpListener;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(_stream) => {
                println!("Server spawned");
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
