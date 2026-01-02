# PicoRDM

[English](README.md) | [简体中文](README_CN.md)

A lightweight, high-performance Redis terminal client built with Rust and Ratatui.

<picture>
 <img alt="screenshot" src="screenshots/1.jpg">
</picture>

## Features

- **High Performance** - Minimal memory footprint with native Rust performance
- **Connection Management** - Multi-connection support with TLS/SSL and Redis Cluster
- **Key Operations** - Browse, search, filter, and manage keys with full data type support
- **Real-time Monitoring** - Live server statistics and metrics
- **Command Execution** - Direct Redis command interface
- **Data Migration** - JSON-based import/export for all Redis data types
- **Customizable UI** - Dark/Light themes with transparency control

## Installation

### Homebrew (macOS/Linux)

```bash
brew install osdodo/picordm/picordm
```

### From Source

```bash
git clone https://github.com/osdodo/picordm.git
cd picordm
cargo build --release
./target/release/picordm
```

## Quick Start

### Connection Setup

Press `i` to import connection from clipboard. Supports both standalone and cluster modes with automatic name generation.

**Standalone Mode:**
```bash
redis://localhost:6379
rediss://user:pass@host:6380
redis-cli -u rediss://user:pass@host:6380 --tls --sni host
```

**Cluster Mode:**
```bash
redis://node1:6379,node2:6379,node3:6379
redis-cli -c -h 127.0.0.1 -p 6379
```

Press `Tab` in the connection form to toggle between modes. See [Cluster Documentation](docs/CLUSTER.md) for setup details.

### Key Bindings

| Key | Action |
|-----|--------|
| `>` | Command mode |
| `/` | Search keys |
| `Esc` | Exit mode |
| `Ctrl+t` | Switch connection |
| `Ctrl+e` | Export data |
| `Ctrl+l` | Import data |
| `Ctrl+n` | Switch database |
| `Ctrl+p` | Settings |
| `F5` | Refresh stats |

**Text Selection:** Hold `Option/Alt` (macOS) or `Shift` (Linux/Windows) while selecting text, then copy using standard shortcuts.

## Documentation

- [Keybindings](docs/KEYBINDINGS.md) - Complete keyboard shortcuts reference
- [Cluster Setup](docs/CLUSTER.md) - Redis Cluster configuration guide
- [Import/Export](docs/IMPORT_DATA.md) - Data import/export format details
