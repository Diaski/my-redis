# my-redis

A Redis-compatible server written in Rust, built as a learning project.

## Features

- RESP protocol parser with full pipelining support
- Commands: `PING`, `SET`, `GET`, `DEL`, `EXISTS`, `EXPIRE`, `TTL`
- Key expiry — lazy (on GET) + active (background task every 1s)
- Multi-threaded via Tokio + DashMap sharding

## Getting Started

```bash
git clone https://github.com/Diaski/my-redis.git
cd my-redis
cargo run --release
```

Connect with `redis-cli`:

```bash
redis-cli -p 6379
```

## Benchmarks

Tested with `redis-benchmark` on Fedora Linux, `cargo run --release`.

| Test                          | SET             | GET             |
| ----------------------------- | --------------- | --------------- |
| Baseline (`-c 50`)            | 96,993 ops/s    | 95,969 ops/s    |
| Pipeline `-P 16`              | 1,315,789 ops/s | 1,449,275 ops/s |
| Pipeline `-P 16 --threads 16` | 3,984,064 ops/s | 4,000,000 ops/s |

**Key finding:** micro-optimizations (`Bytes`, `Cow`, `BytesMut`) gave ~0% throughput gain.
Pipelining gave **14x improvement**. Flamegraph showed ~33% of time spent in kernel TCP (`sendto`) — application code was not the bottleneck.

```

## Roadmap

- [x] Phase 1 — TCP + RESP parser
- [x] Phase 2 — Storage (DashMap)
- [x] Phase 3 — Key expiry (TTL, EXPIRE)
- [ ] Phase 4 — Persistence (RDB / AOF)
- [ ] Phase 5 — Replication

## Built With

- [Tokio](https://tokio.rs) — async runtime
- [DashMap](https://github.com/xacrimon/dashmap) — concurrent HashMap
- [bytes](https://docs.rs/bytes) — zero-copy byte buffers
```
