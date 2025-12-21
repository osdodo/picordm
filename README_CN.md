# PicoRDM

[English](README.md) | [简体中文](README_CN.md)

一个使用 Rust 和 Ratatui 构建的轻量级 Redis 终端管理工具。

<picture>
 <img alt="screenshot" src="screenshots/1.jpg">
</picture>

## 特性

- **轻量级** - 快速性能，最小内存占用
- **连接管理** - 支持多个 Redis 连接和 TLS/SSL
- **Redis 集群** - 完整支持 Redis 集群模式
- **快速连接切换** - 快速切换连接
- **键浏览器** - 搜索、过滤和管理 Redis 键
- **服务器监控** - 实时服务器统计信息
- **命令界面** - 直接执行 Redis 命令
- **导入/导出** - JSON 数据导入/导出，支持所有 Redis 数据类型

## 安装

```bash
cargo build --release
./target/release/picordm
```

## 使用说明

### 连接

- 按 `i` 从剪贴板导入连接
- 在连接表单中按 `Tab` 在单机/集群模式之间切换
- 支持格式：`redis://user:pass@host:port` 和 `redis-cli` 命令格式
- 连接名称会根据主机/首个节点自动生成（例如：`localhost`、`127.0.0.1-cluster`）

**单机模式示例：**

```
redis://localhost:6379
rediss://admin:securepass@prod.redis.com:6380
redis-cli -u rediss://admin:securepass@prod.redis.com:6380 --tls --sni prod.redis.com
```

**集群模式示例：**

```
redis://127.0.0.1:6379,127.0.0.1:6380,127.0.0.1:6381
redis://user:pass@node1.redis.com:6379,node2.redis.com:6379,node3.redis.com:6379
rediss://user:pass@prod1.redis.com:6379,prod2.redis.com:6379,prod3.redis.com:6379
redis-cli -c -h 127.0.0.1 -p 6379
```

对于 `redis-cli -c` 格式，单个节点将作为集群入口点。客户端会自动发现其他节点。

Redis 集群设置请参阅 [集群文档](docs/CLUSTER_CN.md)。

### 键操作

- 在侧边栏浏览和搜索键
- 按 `空格` 选择多个键
- 按 `Delete` 删除选中的键

### 数据导入/导出

- `Ctrl+E` - 导出数据到 JSON 文件
- `Ctrl+L` - 从 JSON 文件导入数据
- 格式详情请参阅 [导入文档](docs/IMPORT_DATA_CN.md)

### 命令

- `>` - 进入命令模式
- `/` - 搜索键
- `Esc` - 退出当前模式
- `Ctrl+t` - 快速连接切换

### 复制文本

要从界面复制文本，请使用终端的文本选择功能：

- **macOS (iTerm2/Terminal)**：按住 `Option/Alt` 键并用鼠标选择文本，然后按 `Cmd+C` 复制
- **Linux**：按住 `Shift` 键并用鼠标选择文本，然后按 `Ctrl+Shift+C` 复制
- **Windows**：按住 `Shift` 键并用鼠标选择文本，然后右键复制

## 文档

- [按键绑定](docs/KEYBINDINGS_CN.md) - 完整的键盘快捷键参考
- [集群设置](docs/CLUSTER_CN.md) - Redis 集群配置指南
- [导入/导出](docs/IMPORT_DATA_CN.md) - 数据导入/导出格式详情
