# Redis Data Import/Export Guide

## Quick Start

### Import Data
1. Press `Ctrl+L` to open file browser
2. Select a JSON file and press Enter
3. Wait for import to complete

### Export Data
1. Press `Ctrl+E` to export data
2. File is automatically saved to desktop

**Note**: You can also select specific keys first, then export only the selected keys instead of all data.

## Data Format

### Basic Format
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

### Fields Description
- `database` (optional): Target database number (0-15). If not specified, imports to currently selected database
- `keys`: Object containing all Redis keys to import

### Data Type Examples

**String**
```json
"user:1": {
  "key_type": "string",
  "value": "John Doe",
  "ttl": null
}
```

**List**
```json
"my_list": {
  "key_type": "list",
  "value": ["item1", "item2", "item3"],
  "ttl": null
}
```

**Hash**
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

**Set**
```json
"my_set": {
  "key_type": "set", 
  "value": ["member1", "member2"],
  "ttl": null
}
```

**Sorted Set**
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

## TTL Settings
- `"ttl": null` - Never expires
- `"ttl": 3600` - Expires after 3600 seconds

## Notes
- Import will not overwrite existing keys
- Only JSON format files are supported
- If `database` field is specified, data will be imported to that database
- If `database` field is omitted, data is imported to the currently selected database
- Export files contain complete metadata information