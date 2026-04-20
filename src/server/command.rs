use crate::server::types::RedisError;
use bytes::Bytes;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::net::tcp::OwnedReadHalf;

const MAX_VALUE_SIZE: usize = 512 * 1024;

pub struct CommandParser;

impl CommandParser {
    pub async fn parse_commands(
        reader: &mut BufReader<OwnedReadHalf>,
    ) -> Result<Vec<Vec<Bytes>>, RedisError> {
        let mut commands = Vec::new();
        let mut line = String::new();

        match Self::parse_single(reader, &mut line).await {
            Ok(cmd) => commands.push(cmd),
            Err(RedisError::Protocol(ref s)) if s == "EOF" => return Ok(commands),
            Err(e) => return Err(e),
        }

        loop {
            if reader.buffer().is_empty() {
                break;
            }
            match Self::parse_single(reader, &mut line).await {
                Ok(cmd) => commands.push(cmd),
                Err(RedisError::Protocol(ref s)) if s == "EOF" => break,
                Err(e) => return Err(e),
            }
        }
        Ok(commands)
    }

    async fn parse_single(
        reader: &mut BufReader<OwnedReadHalf>,
        line: &mut String,
    ) -> Result<Vec<Bytes>, RedisError> {
        line.clear();
        let bytes_read = reader.read_line(line).await.map_err(RedisError::Io)?;
        if bytes_read == 0 {
            return Err(RedisError::Protocol("EOF".to_string()));
        }

        let trimmed = line.trim();
        if !trimmed.starts_with('*') {
            return Err(RedisError::Protocol(format!("Expected '*', found '{}'", trimmed)));
        }

        let n: usize = trimmed[1..]
            .parse()
            .map_err(|_| RedisError::Protocol("Invalid number of arguments".to_string()))?;

        let mut args = Vec::with_capacity(n);
        for _ in 0..n {
            line.clear();
            reader.read_line(line).await.map_err(RedisError::Io)?;
            let trimmed_arg = line.trim();
            if !trimmed_arg.starts_with('$') {
                return Err(RedisError::Protocol("Expected '$'".to_string()));
            }
            let len: usize = trimmed_arg[1..]
                .parse()
                .map_err(|_| RedisError::Protocol("Invalid len".to_string()))?;

            if len > MAX_VALUE_SIZE {
                return Err(RedisError::Protocol("Value too large".to_string()));
            }

            let mut value_buf = vec![0u8; len];
            reader.read_exact(&mut value_buf).await.map_err(RedisError::Io)?;
            let mut term = [0u8; 2];
            reader.read_exact(&mut term).await.map_err(RedisError::Io)?;
            args.push(Bytes::from(value_buf));
        }
        Ok(args)
    }
}
