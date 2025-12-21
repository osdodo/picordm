# PicoRDM

[English](README.md) | [简体中文](README_CN.md)

A lightweight Redis terminal management tool built with Rust and Ratatui.

<picture>
 <img alt="screenshot" src="screenshots/1.jpg">
</picture>

## Features

- **Lightweight** - Fast performance with minimal memory usage
- **Connection Management** - Multiple Redis connections with TLS/SSL support
- **Redis Cluster** - Full support for Redis Cluster mode
- **Quick Connection Switcher** - Fast connection switching
- **Key Browser** - Search, filter, and manage Redis keys
- **Server Monitoring** - Real-time server stats
- **Command Interface** - Execute Redis commands directly
- **Import/Export** - JSON data import/export with full Redis type support

## Installation

```bash
cargo build --release
./target/release/picordm
```

## Note

### Connection

- Press `i` to import connection from clipboard
- Press `Tab` in connection form to switch between Standalone/Cluster mode
- Supports: `redis://user:pass@host:port` and `redis-cli` command formats
- Connection names are auto-generated from host/first node (e.g., `localhost`, `127.0.0.1-cluster`)

**Standalone examples:**

```
redis://localhost:6379
rediss://admin:securepass@prod.redis.com:6380
redis-cli -u rediss://admin:securepass@prod.redis.com:6380 --tls --sni prod.redis.com
```

**Cluster examples:**

```
redis://127.0.0.1:6379,127.0.0.1:6380,127.0.0.1:6381
redis://user:pass@node1.redis.com:6379,node2.redis.com:6379,node3.redis.com:6379
rediss://user:pass@prod1.redis.com:6379,prod2.redis.com:6379,prod3.redis.com:6379
redis-cli -c -h 127.0.0.1 -p 6379
```

For `redis-cli -c` format, a single node is used as the cluster entry point. The client will automatically discover other nodes.

For Redis Cluster setup, see [Cluster Documentation](docs/CLUSTER.md).

### Key Operations

- Browse and search keys in the sidebar
- Press `Space` to select multiple keys
- Press `Delete` to remove selected keys

### Data Import/Export

- `Ctrl+E` - Export data to JSON file
- `Ctrl+L` - Import data from JSON file
- See [IMPORT_DATA.md](docs/IMPORT_DATA.md) for format details

### Commands

- `>` - Enter command mode
- `/` - Search keys
- `Esc` - Exit current mode
- `Ctrl+t` - Quick connection switch

### Copy Text

To copy text from the interface, use your terminal's text selection feature:

- **macOS (iTerm2/Terminal)**: Hold `Option/Alt` key and select text with mouse, then `Cmd+C` to copy
- **Linux**: Hold `Shift` key and select text with mouse, then `Ctrl+Shift+C` to copy
- **Windows**: Hold `Shift` key and select text with mouse, then right-click to copy

## Documentation

- [Keybindings](docs/KEYBINDINGS.md) - Complete keyboard shortcuts reference
- [Cluster Setup](docs/CLUSTER.md) - Redis Cluster configuration guide
- [Import/Export](docs/IMPORT_DATA.md) - Data import/export format details
