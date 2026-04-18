mod command;
mod dispatch;
mod types;
use bytes::Bytes;
use bytes::BytesMut;
use command::Command;
use dashmap::DashMap;
use dispatch::Dispatch;
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use types::{Storage, Timers};
pub struct App {
    listener: TcpListener,
    storage: Storage,
    timers: Timers,
}

impl App {
    pub async fn new(addr: String) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind(addr).await?;
        let storage = Storage::new(DashMap::new());
        let timers = Timers::new(DashMap::new());
        Ok(App { listener, storage, timers })
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
            tokio::spawn(async move {
                Self::server_loop(stream, storage, timers).await;
            });
        }
    }

    async fn server_loop(stream: TcpStream, storage: Storage, timers: Timers) {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        loop {
            match Command::parse_commands(&mut reader).await {
                None => {
                    break;
                }
                Some(commands) => {
                    let mut response_buf = BytesMut::new();
                    for args in commands {
                        let cmd = Command::from_args(args);
                        let response = Dispatch::dispatch(cmd, &storage, &timers);
                        response_buf.extend_from_slice(&response);
                    }
                    if writer.write_all(&response_buf).await.is_err() {
                        break;
                    }
                }
            }
        }
    }
}
