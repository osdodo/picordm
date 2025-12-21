# Redis Cluster Support

picordm supports Redis Cluster mode, allowing you to connect to and manage Redis clusters.

## Quick Start

### Creating a Connection

**Method 1: Manual Entry**

1. Press `n` in the connection list to create a new connection
2. Press `Tab` to switch to **Cluster** mode
3. Enter cluster node addresses (comma-separated)
   ```
   127.0.0.1:6379, 127.0.0.1:6380, 127.0.0.1:6381
   ```
4. Press `Ctrl+S` to save

**Method 2: Quick Import from Clipboard**

1. Copy a cluster connection string to clipboard:
   ```
   redis://127.0.0.1:6379,127.0.0.1:6380,127.0.0.1:6381
   ```
2. Press `i` in the connection list
3. Connection is automatically created with name `127.0.0.1-cluster`

Supported formats:

- `redis://host1:port1,host2:port2,host3:port3` - Multiple nodes (recommended)
- `rediss://user:pass@host1:port1,host2:port2,host3:port3` - With authentication
- `redis-cli -c -h host -p port` - Single entry point (auto-discovers other nodes)

### Form Controls

| Key      | Function                       |
| -------- | ------------------------------ |
| `↑/↓`    | Navigate fields                |
| `Tab`    | Toggle Standalone/Cluster mode |
| `Ctrl+S` | Save connection                |
| `Esc`    | Cancel                         |

## Features

### Supported

- ✅ Automatic cluster redirection handling
- ✅ View and manage keys
- ✅ Execute Redis commands
- ✅ Data import/export
- ✅ TTL management
- ✅ All data types (String, List, Set, ZSet, Hash)

### Limitations

- ❌ SELECT command not supported (automatically ignored)
- ❌ Only db0 is supported
