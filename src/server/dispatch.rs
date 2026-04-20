use super::types::Storage;
use super::types::Timers;
use crate::server::types::{CommandHandler, RedisError};
use bytes::Bytes;
use std::borrow::Cow;
use std::collections::HashMap;
use std::time::Instant;

pub struct CommandRegistry {
    handlers: HashMap<String, Box<dyn CommandHandler>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        let mut handlers: HashMap<String, Box<dyn CommandHandler>> = HashMap::new();
        handlers.insert("PING".to_string(), Box::new(PingHandler));
        handlers.insert("SET".to_string(), Box::new(SetHandler));
        handlers.insert("GET".to_string(), Box::new(GetHandler));
        handlers.insert("DEL".to_string(), Box::new(DelHandler));
        handlers.insert("EXISTS".to_string(), Box::new(ExistsHandler));
        handlers.insert("EXPIRE".to_string(), Box::new(ExpireHandler));
        handlers.insert("TTL".to_string(), Box::new(TtlHandler));

        Self { handlers }
    }

    pub fn handle(&self, args: &[Bytes], storage: &Storage, timers: &Timers) -> Cow<'static, [u8]> {
        if args.is_empty() {
            return Cow::Borrowed(b"-ERR empty command\r\n");
        }

        let cmd_name = std::str::from_utf8(&args[0]).unwrap_or("").to_ascii_uppercase();

        if let Some(handler) = self.handlers.get(&cmd_name) {
            match handler.execute(args, storage, timers) {
                Ok(response) => response,
                Err(e) => Cow::Owned(format!("-ERR {}\r\n", e).into_bytes()),
            }
        } else {
            let unknown = UnknownHandler;
            match unknown.execute(args, storage, timers) {
                Ok(res) => res,
                Err(e) => Cow::Owned(format!("-ERR {}\r\n", e).into_bytes()),
            }
        }
    }
}

pub struct SetHandler;
impl CommandHandler for SetHandler {
    fn execute(
        &self,
        args: &[Bytes],
        storage: &Storage,
        _timers: &Timers,
    ) -> Result<Cow<'static, [u8]>, RedisError> {
        if args.len() < 3 {
            return Err(RedisError::InvalidArg("SET requires key and value".to_string()));
        }
        storage.insert(args[1].clone(), args[2].clone());
        Ok(Cow::Borrowed(b"+OK\r\n"))
    }
}

pub struct GetHandler;
impl CommandHandler for GetHandler {
    fn execute(
        &self,
        args: &[Bytes],
        storage: &Storage,
        timers: &Timers,
    ) -> Result<Cow<'static, [u8]>, RedisError> {
        if args.len() < 2 {
            return Err(RedisError::InvalidArg("GET requires a key".to_string()));
        }
        let key = &args[1];

        if let Some(expire_time) = timers.get(key)
            && std::time::Instant::now() >= *expire_time
        {
            storage.remove(key);
            timers.remove(key);
            return Ok(Cow::Borrowed(b"$-1\r\n"));
        }

        if let Some(val) = storage.get(key) {
            Ok(Cow::Owned(
                format!("${}\r\n{}\r\n", val.len(), std::str::from_utf8(&val).unwrap_or(""))
                    .into_bytes(),
            ))
        } else {
            Ok(Cow::Borrowed(b"$-1\r\n"))
        }
    }
}
pub struct DelHandler;
impl CommandHandler for DelHandler {
    fn execute(
        &self,
        args: &[Bytes],
        storage: &Storage,
        timers: &Timers,
    ) -> Result<Cow<'static, [u8]>, RedisError> {
        if args.len() < 2 {
            return Err(RedisError::InvalidArg("DEL requires a key".to_string()));
        }
        let key = &args[1];
        let r = storage.remove(key);
        match r {
            Some(_) => {
                timers.remove(key);
                Ok(Cow::Borrowed(b":1\r\n"))
            }
            None => Ok(Cow::Borrowed(b":0\r\n")),
        }
    }
}
pub struct ExistsHandler;
impl CommandHandler for ExistsHandler {
    fn execute(
        &self,
        args: &[Bytes],
        storage: &Storage,
        _timers: &Timers,
    ) -> Result<Cow<'static, [u8]>, RedisError> {
        if args.len() < 2 {
            return Err(RedisError::InvalidArg("EXISTS requires a key".to_string()));
        }
        let key = &args[1];
        if storage.contains_key(key) {
            Ok(Cow::Borrowed(b":1\r\n"))
        } else {
            Ok(Cow::Borrowed(b":0\r\n"))
        }
    }
}
pub struct ExpireHandler;
impl CommandHandler for ExpireHandler {
    fn execute(
        &self,
        args: &[Bytes],
        storage: &Storage,
        timers: &Timers,
    ) -> Result<Cow<'static, [u8]>, RedisError> {
        if args.len() < 3 {
            return Err(RedisError::InvalidArg("EXPIRE requires a key and seconds".to_string()));
        }
        let key = &args[1];
        let seconds = std::str::from_utf8(&args[2]).unwrap_or("0").parse().unwrap_or(0);
        if storage.contains_key(key) {
            timers.insert(
                key.clone(),
                std::time::Instant::now() + std::time::Duration::from_secs(seconds),
            );
            Ok(Cow::Borrowed(b":1\r\n"))
        } else {
            Ok(Cow::Borrowed(b":0\r\n"))
        }
    }
}
pub struct TtlHandler;
impl CommandHandler for TtlHandler {
    fn execute(
        &self,
        args: &[Bytes],
        storage: &Storage,
        timers: &Timers,
    ) -> Result<Cow<'static, [u8]>, RedisError> {
        if args.len() < 2 {
            return Err(RedisError::InvalidArg("TTL requires a key".to_string()));
        }
        let key = &args[1];
        if !storage.contains_key(key) {
            return Ok(Cow::Borrowed(b":-2\r\n"));
        }
        match timers.get(key) {
            None => Ok(Cow::Borrowed(b":-1\r\n")),
            Some(expire_time) => match expire_time.checked_duration_since(Instant::now()) {
                None => Ok(Cow::Borrowed(b":-2\r\n")),
                Some(d) => Ok(Cow::Owned(format!(":{}\r\n", d.as_secs()).into_bytes())),
            },
        }
    }
}
pub struct UnknownHandler;
impl CommandHandler for UnknownHandler {
    fn execute(
        &self,
        args: &[Bytes],
        _storage: &Storage,
        _timers: &Timers,
    ) -> Result<Cow<'static, [u8]>, RedisError> {
        if args.is_empty() {
            return Err(RedisError::InvalidArg("Empty command".to_string()));
        }
        let cmd_name = std::str::from_utf8(&args[0]).unwrap_or("");
        Ok(Cow::Owned(format!("-ERR unknown command '{}'\r\n", cmd_name).into_bytes()))
    }
}
pub struct PingHandler;
impl CommandHandler for PingHandler {
    fn execute(
        &self,
        _args: &[Bytes],
        _storage: &Storage,
        _timers: &Timers,
    ) -> Result<Cow<'static, [u8]>, RedisError> {
        Ok(Cow::Borrowed(b"+PONG\r\n"))
    }
}
