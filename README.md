# PicoRDM

A lightweight Redis terminal management tool built with Rust and Ratatui.

## Installation and Usage

```bash
cargo build --release
./target/release/picordm
```

## Quick Import Connection (Shortcut `i`)

1. Copy a Redis connection string to your clipboard
2. Press `i` key in the connection list interface
3. Automatically parse and connect to the Redis server

Supported connection string formats:

```
rediss://admin:securepass@prod.redis.com:6380
```

```
redis-cli -u rediss://admin:securepass@prod.redis.com:6380 --tls --sni admin:securepass@prod.redis.com
```
