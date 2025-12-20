# PicoRDM

A lightweight Redis terminal management tool built with Rust and Ratatui.

<picture>
 <img alt="screenshot" src="screenshots/1.jpg">
</picture>

## Features

- **Lightweight** - Fast performance with minimal memory usage
- **Connection Management** - Multiple Redis connections with TLS/SSL support
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
- Supports: `redis://user:pass@host:port` and `redis-cli` command formats

```
rediss://admin:securepass@prod.redis.com:6380

redis-cli -u rediss://admin:securepass@prod.redis.com:6380 --tls --sni admin:securepass@prod.redis.com
```

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

## TODO

- [ ] Cluster support
