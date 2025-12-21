# Redis 数据导入/导出指南

## 快速开始

### 导入数据

1. 按 `Ctrl+L` 打开文件浏览器
2. 选择一个 JSON 文件并按 Enter
3. 等待导入完成

### 导出数据

1. 按 `Ctrl+E` 导出数据
2. 文件会自动保存到桌面

您也可以先选择特定的键，然后只导出选中的键而不是所有数据。

## 数据格式

### 基本格式

```json
{
  "database": 0,
  "keys": {
    "key_name": {
      "key_type": "string|list|set|zset|hash",
      "value": "data_value",
      "ttl": null
    }
  }
}
```

### 字段说明

- `database`（可选）：目标数据库编号（0-15）。如果未指定，导入到当前选择的数据库
- `keys`：包含所有要导入的 Redis 键的对象

### 数据类型示例

**字符串（String）**

```json
"user:1": {
  "key_type": "string",
  "value": "John Doe",
  "ttl": null
}
```

**列表（List）**

```json
"my_list": {
  "key_type": "list",
  "value": ["item1", "item2", "item3"],
  "ttl": null
}
```

**哈希（Hash）**

```json
"user_profile": {
  "key_type": "hash",
  "value": {
    "name": "John",
    "age": "30"
  },
  "ttl": 3600
}
```

**集合（Set）**

```json
"my_set": {
  "key_type": "set",
  "value": ["member1", "member2"],
  "ttl": null
}
```

**有序集合（Sorted Set）**

```json
"leaderboard": {
  "key_type": "zset",
  "value": {
    "alice": 100,
    "bob": 85
  },
  "ttl": null
}
```

## TTL 设置

- `"ttl": null` - 永不过期
- `"ttl": 3600` - 3600 秒后过期

## 注意事项

- 导入不会覆盖已存在的键
- 仅支持 JSON 格式文件
- 如果指定了 `database` 字段，数据将导入到该数据库
- 如果省略 `database` 字段，数据将导入到当前选择的数据库
- 导出文件包含完整的元数据信息
