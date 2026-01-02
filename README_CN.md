# PicoRDM

[English](README.md) | [简体中文](README_CN.md)

基于 Rust 和 Ratatui 构建的轻量级高性能 Redis 终端客户端。

<picture>
 <img alt="screenshot" src="screenshots/1.jpg">
</picture>

## 特性

- **高性能** - Rust 原生性能，极小内存占用
- **连接管理** - 多连接支持，支持 TLS/SSL 和 Redis 集群
- **键操作** - 浏览、搜索、过滤和管理键，支持所有数据类型
- **实时监控** - 实时服务器统计信息和指标
- **命令执行** - 直接执行 Redis 命令
- **数据迁移** - 基于 JSON 的导入/导出，支持所有 Redis 数据类型
- **自定义界面** - 深色/浅色主题，支持透明度调节

## 安装

### Homebrew (macOS/Linux)

```bash
brew install osdodo/picordm/picordm
```

### 从源码构建

```bash
git clone https://github.com/osdodo/picordm.git
cd picordm
cargo build --release
./target/release/picordm
```

## 快速开始

### 连接配置

按 `i` 从剪贴板导入连接。支持单机和集群模式，自动生成连接名称。

**单机模式：**
```bash
redis://localhost:6379
rediss://user:pass@host:6380
redis-cli -u rediss://user:pass@host:6380 --tls --sni host
```

**集群模式：**
```bash
redis://node1:6379,node2:6379,node3:6379
redis-cli -c -h 127.0.0.1 -p 6379
```

在连接表单中按 `Tab` 切换模式。集群配置详见 [集群文档](docs/CLUSTER_CN.md)。

### 快捷键

| 按键 | 功能 |
|-----|------|
| `>` | 命令模式 |
| `/` | 搜索键 |
| `Esc` | 退出模式 |
| `Ctrl+t` | 切换连接 |
| `Ctrl+e` | 导出数据 |
| `Ctrl+l` | 导入数据 |
| `Ctrl+n` | 切换数据库 |
| `Ctrl+p` | 设置 |
| `F5` | 刷新统计 |

**文本选择：** 按住 `Option/Alt`（macOS）或 `Shift`（Linux/Windows）选择文本，然后使用标准快捷键复制。

## 文档

- [按键绑定](docs/KEYBINDINGS_CN.md) - 完整的键盘快捷键参考
- [集群设置](docs/CLUSTER_CN.md) - Redis 集群配置指南
- [导入/导出](docs/IMPORT_DATA_CN.md) - 数据导入/导出格式详情
