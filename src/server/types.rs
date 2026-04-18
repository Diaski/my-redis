use dashmap::DashMap;
use std::sync::Arc;
pub type Storage = Arc<DashMap<bytes::Bytes, bytes::Bytes>>;
pub type Timers = Arc<DashMap<bytes::Bytes, std::time::Instant>>;
