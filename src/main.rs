use my_redis::server::App;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Starting high-performance my-redis server on 127.0.0.1:6379...");

    let app = App::new("127.0.0.1:6379".to_string()).await.expect("Failed to start server");
    app.run().await;
}
