# my-redis

A high-performance, Redis-compatible key-value store written in Rust. This project was built to explore the limits of asynchronous I/O, memory management, and system-level optimizations in Rust.

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

## Environment:

- **CPU:** AMD Ryzen 7 5700X
- **OS:** Fedora Linux
- **Tool:** redis-benchmark v8.1.6

### Throughput Benchmark

| Scenario           | Configuration                | SET (ops/s)   | GET (ops/s)    | p50 Latency  |
| :----------------- | :--------------------------- | :------------ | :------------- | :----------- |
| **Baseline**       | `-c 50 -P 1 --threads 16`    | **322,289**   | **333,000**    | **~0.12 ms** |
| **Pipelined**      | `-c 50 -P 32 --threads 16`   | **6,657,789** | **7,987,220**  | **~0.17 ms** |
| **Max Throughput** | `-c 500 -P 64 --threads 16`  | **6,648,936** | **13,123,359** | **~1.20 ms** |
| **Large Payload**  | `-d 1024 -P 32 --threads 16` | **3,629,764** | **4,442,470**  | **~0.25 ms** |

### Key Findings

- **Pipelining Impact:** Moving from single requests to a pipeline of 32 requests increased throughput by **~20x**.
- **Payload Scalability:** Even with a 1KB payload, the server maintains a massive throughput (~4M ops/s for GET), proving the efficiency of the zero-copy architecture.
- **Stability:** The p99 latency remains extremely low (sub-millisecond), indicating minimal jitter and efficient memory reclamation.

## 🛠 Engineering & Optimizations

To achieve this level of performance, several low-level optimizations were implemented:

### 1. Zero-Copy RESP Parsing

Instead of using `read_line` or `String` conversions (which involve UTF-8 validation and multiple allocations), I implemented a custom byte-level parser. It operates directly on `&[u8]` slices, using a sliding window buffer (`BytesMut`) to eliminate unnecessary data copying.

### 2. Advanced Memory Management

- **mimalloc**: Integrated Microsoft's `mimalloc` as the global allocator to reduce fragmentation and speed up small object allocations.
- **Buffer Recycling**: Implemented a response buffer recycling strategy to avoid allocating new buffers for every single request, drastically reducing pressure on the allocator.
- **Bytes Crate**: Utilized `bytes::Bytes` for efficient reference-counted byte buffers, allowing data to be shared across threads without deep copies.

### 3. Concurrency Model

- **Sharded Storage**: Used `DashMap` to minimize lock contention. By sharding the data, multiple threads can read and write to different parts of the database simultaneously.
- **Tokio Runtime**: Leveraged the `tokio` multi-threaded runtime for asynchronous I/O handling.
- **TCP Tuning**: Enabled `TCP_NODELAY` to disable Nagle's algorithm, ensuring that small Redis packets are sent immediately without artificial delays.

### 4. System-Level Tuning

The server was tuned for Linux to maximize network throughput:

- Increased `somaxconn` and `tcp_max_syn_backlog` for higher connection rates.
- Optimized TCP buffer sizes (`rmem` and `wmem`) for high-bandwidth traffic.
- Tuned `memlock` limits to optimize memory pinning for the network stack.

## Features

- **RESP Protocol**: Full implementation of the Redis Serialization Protocol.
- **Core Commands**: `PING`, `SET`, `GET`, `DEL`, `EXISTS`, `EXPIRE`, `TTL`.
- **TTL Engine**: Hybrid expiration strategy combining lazy deletion (on access) and active background cleaning.
- **Pipelining**: Full support for request pipelining.

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
- [mimalloc](https://github.com/microsoft/mimalloc) — High-performance memory allocator
