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

## Benchmarks

**Environment:**

- **CPU:** AMD Ryzen 7 5700X (16 cores)
- **OS:** Fedora Linux
- **Tool:** valkey-benchmark v8.1.6
- **Comparison:** Redis 7.x (single-threaded) vs my-redis (multi-threaded, Tokio + DashMap)

---

### Throughput — my-redis

| Scenario                    | Configuration                | SET (ops/s) | GET (ops/s) |     p50 |
| :-------------------------- | :--------------------------- | ----------: | ----------: | ------: |
| Baseline                    | `-c 50 -P 1 --threads 16`    |     322,289 |     333,000 | ~0.12ms |
| Pipelined                   | `-c 50 -P 32 --threads 16`   |   6,657,789 |   7,987,220 | ~0.17ms |
| Max Throughput              | `-c 500 -P 64 --threads 16`  |   6,648,936 |  13,123,359 | ~1.20ms |
| Large Payload (1KB)         | `-d 1024 -P 32 --threads 16` |   3,629,764 |   4,442,470 | ~0.25ms |
| Stress Test (100M requests) | `-c 1000 -P 16 --threads 8`  |   6,333,122 |   6,433,350 |  ~1.5ms |

---

### my-redis vs Redis — scaling with client count (no pipelining)

| Clients | Redis SET | my-redis SET | Advantage | Redis p50 | my-redis p50 |
| ------: | --------: | -----------: | --------: | --------: | -----------: |
|       1 |    41,621 |       44,393 |      1.1x |   0.023ms |      0.023ms |
|      10 |   133,155 |      249,501 |  **1.9x** |   0.071ms |      0.031ms |
|      50 |   133,226 |      332,889 |  **2.5x** |   0.359ms |      0.111ms |
|     100 |   133,155 |      399,361 |    **3x** |   0.719ms |      0.199ms |
|     200 |   133,014 |      399,361 |    **3x** |   1.463ms |      0.351ms |

Redis hits a ceiling at ~133k ops/sec with 10+ clients — the limit of its single-threaded model. my-redis scales linearly up to ~400k.

---

### my-redis vs Redis — scaling with pipeline depth (`-c 50 --threads 16`)

| Pipeline | Redis SET | my-redis SET | Advantage |
| -------: | --------: | -----------: | --------: |
|        1 |   128,915 |      333,222 |  **2.6x** |
|        4 |   444,049 |    1,329,787 |    **3x** |
|        8 |   666,222 |    1,996,008 |    **3x** |
|       16 |   998,004 |    3,984,064 |    **4x** |
|       32 | 1,329,787 |    3,984,064 |    **3x** |

---

### Key Findings

- **At 1 client, both servers are equal** (~41-44k ops/sec) — per-operation overhead in Rust matches Redis written in C.
- **Pipelining gave a 14x improvement** over the baseline. All other micro-optimizations (`Bytes`, `Cow`, `BytesMut`) gave ~0% throughput gain.
- **Flamegraph showed ~33% of time in kernel TCP** (`sendto`) — application code was not the bottleneck.
- **my-redis beats Redis 4x at pipeline depth 16** due to multi-threading. Redis still grows at pipeline 32 because it has headroom left on its single thread.

---

## Engineering & Optimizations

### 1. Zero-Copy RESP Parsing

Instead of `read_line` or `String` conversions, a custom byte-level parser operates directly on `&[u8]` slices using a sliding window `BytesMut` buffer — no unnecessary copies or UTF-8 validation during parsing.

### 2. Memory Management

- **mimalloc** — Microsoft's allocator as the global allocator, reducing fragmentation and speeding up small object allocations.
- **Buffer recycling** — response buffer reused across requests, avoiding per-request allocation.
- **`bytes::Bytes`** — reference-counted byte buffers shared across threads without deep copies.
- **`Cow<'static, [u8]>`** — static responses (PONG, OK, errors) allocated once at compile time, zero runtime allocation.

### 3. Concurrency Model

- **DashMap** — sharded HashMap with `with_capacity` pre-allocation, minimizing lock contention and rehashing spikes.
- **Tokio** — work-stealing multi-threaded async runtime.
- **TCP_NODELAY** — Nagle's algorithm disabled, small packets sent immediately.

### 4. System-Level Tuning

- Increased `somaxconn` and `tcp_max_syn_backlog` for higher connection rates.
- Optimized TCP buffer sizes (`rmem` / `wmem`) for high-bandwidth traffic.

---

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
- [mimalloc](https://github.com/microsoft/mimalloc) — high-performance memory allocator
