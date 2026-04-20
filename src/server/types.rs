use bytes::Bytes;
use dashmap::DashMap;
use std::borrow::Cow;
use std::sync::Arc;

pub type Storage = Arc<DashMap<Bytes, Bytes>>;
pub type Timers = Arc<DashMap<Bytes, std::time::Instant>>;

pub trait CommandHandler: Send + Sync {
    fn execute(
        &self,
        args: &[Bytes],
        storage: &Storage,
        timers: &Timers,
    ) -> Result<Cow<'static, [u8]>, RedisError>;
}

#[derive(thiserror::Error, Debug)]
pub enum RedisError {
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid argument: {0}")]
    InvalidArg(String),
}
