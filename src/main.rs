use my_redis::server::App;

#[tokio::main]
async fn main() {
    let app = App::new("127.0.0.1:6379".to_string()).await.expect("Failed to start server");
    app.run().await;
}
