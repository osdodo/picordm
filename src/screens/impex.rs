use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::service::get_redis_service;

#[derive(Debug)]
pub struct ExportResult {
    pub exported_count: usize,
    pub failed_keys: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct ImportResult {
    pub imported_count: usize,
    pub skipped_count: usize,
    pub failed_keys: Vec<(String, String)>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedisExportData {
    pub version: String,
    pub export_time: String,
    pub connection_name: String,
    pub database: u32,
    pub keys: HashMap<String, RedisKeyData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedisImportData {
    #[serde(default)]
    pub database: Option<u32>,
    pub keys: HashMap<String, RedisKeyData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedisKeyData {
    pub key_type: String,
    pub value: serde_json::Value,
    pub ttl: Option<i64>, // TTL in seconds, None means no expiration
}

impl RedisExportData {
    pub fn new(connection_name: String, database: u32) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            export_time: chrono::Local::now().to_rfc3339(),
            connection_name,
            database,
            keys: HashMap::new(),
        }
    }
}

pub async fn export_redis_data(
    connection_name: String,
    database: u32,
    keys: &[String],
    file_path: &Path,
) -> Result<ExportResult> {
    let mut export_data = RedisExportData::new(connection_name, database);
    let mut exported_count = 0;
    let mut failed_keys = Vec::new();

    for key in keys {
        match export_redis_key(key, database).await {
            Ok(key_data) => {
                export_data.keys.insert(key.clone(), key_data);
                exported_count += 1;
            }
            Err(e) => {
                failed_keys.push((key.clone(), e.to_string()));
            }
        }
    }

    let json_content = serde_json::to_string_pretty(&export_data)
        .context("Failed to serialize Redis data to JSON")?;

    fs::write(file_path, json_content)
        .with_context(|| format!("Failed to write Redis export file: {:?}", file_path))?;

    Ok(ExportResult {
        exported_count,
        failed_keys,
    })
}

async fn export_redis_key(key: &str, database: u32) -> Result<RedisKeyData> {
    let redis = get_redis_service();
    let key_type = redis.get_type(key, database).await?;
    let ttl = redis.get_key_ttl(key, database).await.ok();
    let value = match key_type.as_str() {
        "string" => {
            let val = redis.get_value(key, database).await?;
            serde_json::Value::String(val)
        }
        "list" => {
            let vals = redis.get_list_values(key, database).await?;
            serde_json::Value::Array(vals.into_iter().map(serde_json::Value::String).collect())
        }
        "set" => {
            let vals = redis.get_set_values(key, database).await?;
            serde_json::Value::Array(vals.into_iter().map(serde_json::Value::String).collect())
        }
        "zset" => {
            let vals = redis.get_zset_values(key, database).await?;
            let obj: serde_json::Map<String, serde_json::Value> = vals
                .into_iter()
                .map(|(member, score)| {
                    (
                        member,
                        serde_json::Value::Number(
                            serde_json::Number::from_f64(score)
                                .unwrap_or_else(|| serde_json::Number::from(0)),
                        ),
                    )
                })
                .collect();
            serde_json::Value::Object(obj)
        }
        "hash" => {
            let vals = redis.get_hash_values(key, database).await?;
            let obj: serde_json::Map<String, serde_json::Value> = vals
                .into_iter()
                .map(|(k, v)| (k, serde_json::Value::String(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
        _ => {
            // Fallback to string for unknown types
            let val = redis.get_value(key, database).await?;
            serde_json::Value::String(val)
        }
    };

    Ok(RedisKeyData {
        key_type,
        value,
        ttl,
    })
}

pub async fn import_redis_data(
    file_path: &Path,
    database: u32,
    overwrite: bool,
) -> Result<ImportResult> {
    let redis = get_redis_service();
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read Redis import file: {:?}", file_path))?;

    // Try to parse as flexible import format first, fallback to export format
    let import_data = match serde_json::from_str::<RedisImportData>(&content) {
        Ok(data) => data,
        Err(_) => {
            // Fallback: try to parse as full export format
            let export_data: RedisExportData = serde_json::from_str(&content)
                .context("Failed to parse JSON from Redis import file")?;
            RedisImportData {
                database: Some(export_data.database),
                keys: export_data.keys,
            }
        }
    };

    let mut imported_count = 0;
    let mut skipped_count = 0;
    let mut failed_keys = Vec::new();

    // Use database from JSON file if specified, otherwise use the provided database parameter
    let target_database = import_data.database.unwrap_or(database);

    for (key, key_data) in import_data.keys {
        // Check if key exists
        if !overwrite
            && redis
                .key_exists(&key, target_database)
                .await
                .unwrap_or(false)
        {
            skipped_count += 1;
            continue;
        }

        match import_redis_key(&key, &key_data, target_database).await {
            Ok(_) => {
                imported_count += 1;

                // Set TTL if specified
                if let Some(ttl) = key_data.ttl
                    && ttl > 0
                {
                    let _ = redis.set_key_ttl(&key, ttl, target_database).await;
                }
            }
            Err(e) => {
                failed_keys.push((key.clone(), e.to_string()));
                skipped_count += 1;
            }
        }
    }

    Ok(ImportResult {
        imported_count,
        skipped_count,
        failed_keys,
    })
}

async fn import_redis_key(key: &str, key_data: &RedisKeyData, database: u32) -> Result<()> {
    let redis = get_redis_service();
    match key_data.key_type.as_str() {
        "string" => {
            if let serde_json::Value::String(val) = &key_data.value {
                redis.set_value(key, val, database).await?;
            }
        }
        "list" => {
            if let serde_json::Value::Array(vals) = &key_data.value {
                let string_vals: Vec<String> = vals
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                redis.set_list_values(key, &string_vals, database).await?;
            }
        }
        "set" => {
            if let serde_json::Value::Array(vals) = &key_data.value {
                let string_vals: Vec<String> = vals
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                redis.set_set_values(key, &string_vals, database).await?;
            }
        }
        "zset" => {
            if let serde_json::Value::Object(vals) = &key_data.value {
                let scored_vals: Vec<(String, f64)> = vals
                    .iter()
                    .filter_map(|(member, score)| score.as_f64().map(|s| (member.clone(), s)))
                    .collect();
                redis.set_zset_values(key, &scored_vals, database).await?;
            }
        }
        "hash" => {
            if let serde_json::Value::Object(vals) = &key_data.value {
                let hash_vals: Vec<(String, String)> = vals
                    .iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect();
                redis.set_hash_values(key, &hash_vals, database).await?;
            }
        }
        _ => {
            // Fallback to string
            if let serde_json::Value::String(val) = &key_data.value {
                redis.set_value(key, val, database).await?;
            }
        }
    }

    Ok(())
}

pub fn get_export_path(default_name: &str) -> std::path::PathBuf {
    // Try to use desktop directory, fallback to current directory
    if let Some(desktop_dir) = dirs::desktop_dir() {
        desktop_dir.join(default_name)
    } else {
        // Fallback to current directory if desktop path is not available
        let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        current_dir.join(default_name)
    }
}
