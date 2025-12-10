# PicoRDM

A lightweight Redis terminal management tool built with Rust and Ratatui.

<picture>
 <img alt="screenshot" src="screenshots/1.jpg">
</picture>

## Features

- 🪶 **Lightweight** - Minimal memory footprint and fast performance
- 🔌 **Connection Management** - Save and manage multiple Redis connections with TLS/SSL support
- 📋 **Quick Import** - One-click import from clipboard connection strings (shortcut `i`)
- 🔍 **Key Browser** - Real-time search and filter Redis keys with batch selection and deletion
- 📊 **Server Monitoring** - Display uptime, memory usage, connected clients, and key counts
- 💾 **Database Switching** - Seamlessly switch between Redis databases
- ⌨️ **Command Interface** - Execute Redis commands directly in the TUI with `>` shortcut


## Installation and Usage

```bash
cargo build --release
./target/release/picordm
```

## Note

### Quick Import Connection (Shortcut `i`)

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

### Copy Text

To copy text from the interface, use your terminal's text selection feature:
- **macOS (iTerm2/Terminal)**: Hold `Option/Alt` key and select text with mouse, then `Cmd+C` to copy
- **Linux**: Hold `Shift` key and select text with mouse, then `Ctrl+Shift+C` to copy
- **Windows**: Hold `Shift` key and select text with mouse, then right-click to copy

### Execute Redis Commands (CLI Mode)
- Press `>` to enter command mode (switches to CLI interface)
- Type any Redis command directly (e.g., `GET mykey`, `SET key value`, `KEYS *`, `DBSIZE`, `INFO`)

## TODO

- [ ] Import and export functionality
- [ ] Quick connection switching
