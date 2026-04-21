mod command;
mod dispatch;
mod types;

use bytes::{Bytes, BytesMut};
use command::CommandParser;
use dashmap::DashMap;
use dispatch::CommandRegistry;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use types::{Storage, Timers};

pub struct App {
    listener: TcpListener,
    storage: Storage,
    timers: Timers,
    registry: Arc<CommandRegistry>,
}

impl App {
    pub async fn new(addr: String) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind(addr).await?;
        let storage = Storage::new(DashMap::with_capacity(1_000_000));
        let timers = Timers::new(DashMap::with_capacity(1_000_000));
        let registry = Arc::new(CommandRegistry::new());
        Ok(App { listener, storage, timers, registry })
    }

    pub fn local_addr(&self) -> std::net::SocketAddr {
        self.listener.local_addr().unwrap()
    }

    fn expire_keys_loop(storage: Storage, timers: Timers) {
        let now = std::time::Instant::now();
        let expired_keys: Vec<Bytes> = timers
            .iter()
            .filter_map(
                |entry| {
                    if *entry.value() <= now { Some(entry.key().clone()) } else { None }
                },
            )
            .collect();
        for key in expired_keys {
            storage.remove(&key);
            timers.remove(&key);
        }
    }

    pub async fn run(self) {
        let storage_bg = self.storage.clone();
        let timers_bg = self.timers.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                Self::expire_keys_loop(storage_bg.clone(), timers_bg.clone());
            }
        });

        while let Ok((stream, _addr)) = self.listener.accept().await {
            let storage = self.storage.clone();
            let timers = self.timers.clone();
            let registry = self.registry.clone();
            tokio::spawn(async move {
                Self::server_loop(stream, storage, timers, registry).await;
            });
        }
    }

    async fn server_loop(
        stream: TcpStream,
        storage: Storage,
        timers: Timers,
        registry: Arc<CommandRegistry>,
    ) {
        stream.set_nodelay(true).expect("set_nodelay failed");

        let (mut reader, mut writer) = stream.into_split();

        let mut read_buf = BytesMut::with_capacity(16384);
        let mut response_buf = BytesMut::with_capacity(4096);
        loop {
            read_buf.reserve(8192);
            match reader.read_buf(&mut read_buf).await {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => {
                    if let Some(io_err) = e.raw_os_error()
                        && io_err == 104
                    {
                        tracing::debug!("Client reset connection");
                        break;
                    }

                    tracing::error!("Read error: {:?}", e);
                    break;
                }
            };

            match CommandParser::parse_commands(&mut read_buf) {
                Ok(commands) => {
                    if commands.is_empty() {
                        continue;
                    }

                    response_buf.clear();
                    for args in commands {
                        let response = registry.handle(&args, &storage, &timers);
                        response_buf.extend_from_slice(&response);
                    }

                    if writer.write_all(&response_buf).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Fatal protocol error from client: {:?}. Closing connection.",
                        e
                    );
                    break;
                }
            }
        }
    }
}
