use bytes::Bytes;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::tcp::OwnedReadHalf;

pub enum Command {
    Ping,
    Get { key: Bytes },
    Set { key: Bytes, value: Bytes },
    Del { key: Bytes },
    Exists { key: Bytes },
    Expire { key: Bytes, seconds: u64 },
    Ttl { key: Bytes },
    Unknown(Bytes),
}

impl Command {
    pub fn from_args(args: Vec<Bytes>) -> Self {
        if args.is_empty() {
            return Command::Unknown(Bytes::from("empty command"));
        }

        let cmd = std::str::from_utf8(&args[0]).unwrap_or("").to_ascii_uppercase();

        match cmd.as_str() {
            "PING" => Command::Ping,
            "SET" => {
                if args.len() < 3 {
                    Command::Unknown(Bytes::from("wrong number of arguments for 'set' command"))
                } else {
                    Command::Set { key: args[1].clone(), value: args[2].clone() }
                }
            }
            "GET" => {
                if args.len() < 2 {
                    Command::Unknown(Bytes::from("wrong number of arguments for 'get' command"))
                } else {
                    Command::Get { key: args[1].clone() }
                }
            }
            "DEL" => Command::Del { key: args[1].clone() },
            "EXISTS" => Command::Exists { key: args[1].clone() },
            "EXPIRE" => {
                if args.len() < 3 {
                    Command::Unknown(Bytes::from("wrong number of arguments for 'expire' command"))
                } else {
                    let seconds = std::str::from_utf8(&args[2]).unwrap_or("0").parse().unwrap_or(0);
                    Command::Expire { key: args[1].clone(), seconds }
                }
            }
            "TTL" => Command::Ttl { key: args[1].clone() },
            other => Command::Unknown(Bytes::copy_from_slice(other.as_bytes())),
        }
    }

    pub async fn parse_commands(reader: &mut BufReader<OwnedReadHalf>) -> Option<Vec<Vec<Bytes>>> {
        let mut commands = Vec::new();
        let mut buf = String::with_capacity(256);
        let first = Self::parse_single(reader, &mut buf).await?;
        commands.push(first);
        loop {
            if reader.buffer().is_empty() {
                break;
            }
            match Self::parse_single(reader, &mut buf).await {
                Some(cmd) => commands.push(cmd),
                None => break,
            }
        }

        Some(commands)
    }

    async fn parse_single(
        reader: &mut BufReader<OwnedReadHalf>,
        buf: &mut String,
    ) -> Option<Vec<Bytes>> {
        buf.clear();
        reader.read_line(buf).await.ok()?;
        let n: usize = buf.trim().strip_prefix('*')?.parse().ok()?;

        let mut args = Vec::with_capacity(n);
        for _ in 0..n {
            buf.clear();
            reader.read_line(buf).await.ok()?;
            buf.clear();
            reader.read_line(buf).await.ok()?;
            args.push(Bytes::copy_from_slice(buf.trim().as_bytes()));
        }

        Some(args)
    }
}
