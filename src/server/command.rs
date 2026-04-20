use crate::server::types::RedisError;
use bytes::{Buf, Bytes, BytesMut};

const MAX_VALUE_SIZE: usize = 512 * 1024;

pub struct CommandParser;

impl CommandParser {
    pub fn parse_commands(buf: &mut BytesMut) -> Result<Vec<Vec<Bytes>>, RedisError> {
        let mut commands = Vec::new();

        loop {
            match Self::parse_single(buf) {
                Ok(cmd) => commands.push(cmd),
                Err(RedisError::Protocol(s)) if s == "INCOMPLETE" => break,
                Err(e) => {
                    return Err(e);
                }
            }
        }
        Ok(commands)
    }

    fn parse_single(buf: &mut BytesMut) -> Result<Vec<Bytes>, RedisError> {
        if buf.is_empty() {
            return Err(RedisError::Protocol("INCOMPLETE".to_string()));
        }

        if buf[0] != b'*' {
            return Err(RedisError::Protocol("Expected '*'".to_string()));
        }

        let mut pos = 1;
        let mut n = 0usize;
        while pos < buf.len() && buf[pos] != b'\r' {
            if buf[pos].is_ascii_digit() {
                n = n * 10 + (buf[pos] - b'0') as usize;
            } else {
                return Err(RedisError::Protocol("Invalid count".to_string()));
            }
            pos += 1;
        }

        if pos + 2 > buf.len() {
            return Err(RedisError::Protocol("INCOMPLETE".to_string()));
        }
        let header_end = pos + 2;

        let mut args = Vec::with_capacity(n);
        let mut current_pos = header_end;

        for _ in 0..n {
            if current_pos >= buf.len() {
                return Err(RedisError::Protocol("INCOMPLETE".to_string()));
            }
            if buf[current_pos] != b'$' {
                return Err(RedisError::Protocol("Expected '$'".to_string()));
            }

            let mut pos_len = current_pos + 1;
            let mut len = 0usize;
            while pos_len < buf.len() && buf[pos_len] != b'\r' {
                if buf[pos_len].is_ascii_digit() {
                    len = len * 10 + (buf[pos_len] - b'0') as usize;
                } else {
                    return Err(RedisError::Protocol("Invalid length".to_string()));
                }
                pos_len += 1;
            }

            if pos_len + 2 > buf.len() {
                return Err(RedisError::Protocol("INCOMPLETE".to_string()));
            }
            let len_end = pos_len + 2;

            if len > MAX_VALUE_SIZE {
                return Err(RedisError::Protocol("Value too large".to_string()));
            }

            let data_start = len_end;
            let data_end = data_start + len;
            if data_end + 2 > buf.len() {
                return Err(RedisError::Protocol("INCOMPLETE".to_string()));
            }

            args.push(Bytes::copy_from_slice(&buf[data_start..data_end]));
            current_pos = data_end + 2;
        }

        buf.advance(current_pos);
        Ok(args)
    }
}
