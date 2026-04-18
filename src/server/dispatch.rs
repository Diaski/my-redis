use super::command::Command;
use super::types::Storage;
use super::types::Timers;
use std::borrow::Cow;
use std::time::Instant;
pub struct Dispatch;

impl Dispatch {
    pub fn dispatch(cmd: Command, storage: &Storage, timers: &Timers) -> Cow<'static, [u8]> {
        match cmd {
            Command::Ping => Cow::Borrowed(b"+PONG\r\n"),
            Command::Set { key, value } => {
                storage.insert(key, value);
                Cow::Borrowed(b"+OK\r\n")
            }
            Command::Get { key } => {
                if let Some(expire_time) = timers.get(&key)
                    && Instant::now() >= *expire_time
                {
                    storage.remove(&key);
                    timers.remove(&key);
                    return Cow::Borrowed(b"$-1\r\n");
                }
                if let Some(val) = storage.get(&key) {
                    Cow::Owned(
                        format!(
                            "${}\r\n{}\r\n",
                            val.len(),
                            std::str::from_utf8(&val).unwrap_or("")
                        )
                        .into_bytes(),
                    )
                } else {
                    Cow::Borrowed(b"$-1\r\n")
                }
            }
            Command::Del { key } => {
                let r = storage.remove(&key);
                match r {
                    Some(_) => {
                        timers.remove(&key);
                        Cow::Borrowed(b":1\r\n")
                    }
                    None => Cow::Borrowed(b":0\r\n"),
                }
            }
            Command::Exists { key } => {
                if let Some(expire_time) = timers.get(&key)
                    && Instant::now() >= *expire_time
                {
                    storage.remove(&key);
                    timers.remove(&key);
                    return Cow::Borrowed(b":0\r\n");
                }
                if storage.contains_key(&key) {
                    Cow::Borrowed(b":1\r\n")
                } else {
                    Cow::Borrowed(b":0\r\n")
                }
            }
            Command::Expire { key, seconds } => {
                if storage.contains_key(&key) {
                    timers.insert(
                        key,
                        std::time::Instant::now() + std::time::Duration::from_secs(seconds),
                    );
                    Cow::Borrowed(b":1\r\n")
                } else {
                    Cow::Borrowed(b":0\r\n")
                }
            }
            Command::Ttl { key } => {
                if !storage.contains_key(&key) {
                    return Cow::Borrowed(b":-2\r\n");
                }
                match timers.get(&key) {
                    None => Cow::Borrowed(b":-1\r\n"),
                    Some(expire_time) => match expire_time.checked_duration_since(Instant::now()) {
                        None => Cow::Borrowed(b":-2\r\n"),
                        Some(d) => Cow::Owned(format!(":{}\r\n", d.as_secs()).into_bytes()),
                    },
                }
            }
            Command::Unknown(msg) => Cow::Owned(
                format!("-ERR {}\r\n", std::str::from_utf8(&msg).unwrap_or("")).into_bytes(),
            ),
        }
    }
}
