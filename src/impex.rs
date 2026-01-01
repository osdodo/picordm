use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result};
use futures::stream::{FuturesUnordered, StreamExt};
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
pub struct RedisImportData {
    #[serde(default)]
    pub database: Option<u32>,
    pub keys: HashMap<String, RedisKeyData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisKeyData {
    pub key_type: String,
    pub value: serde_json::Value,
    pub ttl: Option<i64>, // TTL in seconds, None means no expiration
}

pub async fn export_redis_data<F>(
    connection_name: String,
    database: u32,
    keys: &[String],
    file_path: &Path,
    progress_callback: Option<F>,
) -> Result<ExportResult>
where
    F: Fn(usize, usize) + Send + Sync,
{
    let mut exported_count = 0;
    let mut failed_keys = Vec::new();

    // Create a file and write header information
    let file = File::create(file_path)
        .with_context(|| format!("Failed to create Redis export file: {:?}", file_path))?;
    let mut writer = BufWriter::new(file);

    // Write JSON header
    writeln!(writer, "{{")?;
    writeln!(writer, r#"  "version": "{}","#, env!("CARGO_PKG_VERSION"))?;
    writeln!(
        writer,
        r#"  "export_time": "{}","#,
        chrono::Local::now().to_rfc3339()
    )?;
    writeln!(
        writer,
        r#"  "connection_name": {},"#,
        serde_json::to_string(&connection_name)?
    )?;
    writeln!(writer, r#"  "database": {},"#, database)?;
    writeln!(writer, r#"  "keys": {{"#)?;

    // Concurrent export, processing 10 keys per batch
    const BATCH_SIZE: usize = 10;
    let mut is_first_key = true;
    let total_keys = keys.len();

    for chunk in keys.chunks(BATCH_SIZE) {
        // Batch retrieve the type and TTL of all keys (optimized using pipeline)
        let redis = get_redis_service();
        let keys_vec: Vec<String> = chunk.iter().map(|s| s.to_string()).collect();
        let metadata = redis.get_keys_metadata(&keys_vec, database).await?;

        // Use FuturesUnordered to handle tasks; processing occurs as soon as the task is completed.
        let mut tasks = FuturesUnordered::new();
        for (i, key) in chunk.iter().enumerate() {
            let key_clone = key.clone();
            let (key_type, ttl) = metadata
                .get(i)
                .cloned()
                .unwrap_or(("string".to_string(), None));

            tasks.push(tokio::spawn(async move {
                (
                    key_clone.clone(),
                    export_redis_key_with_metadata(&key_clone, database, key_type, ttl).await,
                )
            }));
        }

        // Process tasks as soon as they are completed, without waiting for the entire batch to finish.
        while let Some(task_result) = tasks.next().await {
            match task_result {
                Ok((key, Ok(key_data))) => {
                    // Write comma separators (except for the first key)
                    if !is_first_key {
                        writeln!(writer, ",")?;
                    }
                    is_first_key = false;

                    // Serialize and write to a single key (without retaining the entire data in memory)
                    let key_json = serde_json::to_string(&key)?;
                    let key_data_json = serde_json::to_string_pretty(&key_data)?;
                    write!(writer, "    {}: {}", key_json, key_data_json)?;

                    exported_count += 1;

                    // Call the progress callback immediately
                    if let Some(ref callback) = progress_callback {
                        callback(exported_count, total_keys);
                    }
                }
                Ok((key, Err(e))) => {
                    failed_keys.push((key, e.to_string()));
                }
                Err(e) => {
                    failed_keys.push(("unknown".to_string(), e.to_string()));
                }
            }
        }
    }

    // Write to the end of JSON
    writeln!(writer)?;
    writeln!(writer, "  }}")?;
    writeln!(writer, "}}")?;

    writer.flush()?;

    Ok(ExportResult {
        exported_count,
        failed_keys,
    })
}

async fn export_redis_key_with_metadata(
    key: &str,
    database: u32,
    key_type: String,
    ttl: Option<i64>,
) -> Result<RedisKeyData> {
    let redis = get_redis_service();
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

pub async fn import_redis_data<F>(
    file_path: &Path,
    database: u32,
    overwrite: bool,
    progress_callback: Option<F>,
) -> Result<ImportResult>
where
    F: Fn(usize, usize) + Send + Sync,
{
    // Check file size (streaming read can handle larger files)
    const WARN_SIZE_MB: u64 = 200;

    let metadata = fs::metadata(file_path)
        .with_context(|| format!("Failed to read file metadata: {:?}", file_path))?;
    let file_size_mb = metadata.len() / (1024 * 1024);

    if file_size_mb > WARN_SIZE_MB {
        eprintln!(
            "⚠️  Warning: Large file detected ({} MB)\n\
            Using streaming import for better memory efficiency.\n\
            This may take a while...\n",
            file_size_mb
        );
    }

    // Use streaming read to avoid loading the entire file into memory at once
    let file = File::open(file_path)
        .with_context(|| format!("Failed to open Redis import file: {:?}", file_path))?;
    let reader = BufReader::new(file);

    let import_data: RedisImportData =
        serde_json::from_reader(reader).context("Failed to parse JSON from Redis import file")?;

    let mut imported_count = 0;
    let mut skipped_count = 0;
    let mut failed_keys = Vec::new();

    // Use database from JSON file if specified, otherwise use the provided database parameter
    let target_database = import_data.database.unwrap_or(database);

    // Concurrent import, processing 10 keys per batch
    const BATCH_SIZE: usize = 10;
    let keys_vec: Vec<_> = import_data.keys.into_iter().collect();
    let total_keys = keys_vec.len();

    // Initialize progress to 0
    if let Some(ref callback) = progress_callback {
        callback(0, total_keys);
    }

    for chunk in keys_vec.chunks(BATCH_SIZE) {
        let mut tasks = Vec::new();

        for (key, key_data) in chunk {
            let key_clone = key.clone();
            let key_data_clone = key_data.clone();

            tasks.push(tokio::spawn(async move {
                // Check if key exists
                let exists = if !overwrite {
                    get_redis_service()
                        .key_exists(&key_clone, target_database)
                        .await
                        .unwrap_or(false)
                } else {
                    false
                };

                if exists {
                    return (
                        key_clone,
                        Err(anyhow::anyhow!("Key exists and overwrite is false")),
                    );
                }

                let result = import_redis_key(&key_clone, &key_data_clone, target_database).await;

                // Set TTL if specified
                if result.is_ok()
                    && let Some(ttl) = key_data_clone.ttl
                    && ttl > 0
                {
                    let _ = get_redis_service()
                        .set_key_ttl(&key_clone, ttl, target_database)
                        .await;
                }

                (key_clone, result)
            }));
        }

        // Wait for the current batch to complete
        for task in tasks {
            match task.await {
                Ok((_, Ok(_))) => {
                    imported_count += 1;

                    if let Some(ref callback) = progress_callback {
                        callback(imported_count + skipped_count, total_keys);
                    }
                }
                Ok((key, Err(e))) => {
                    if e.to_string().contains("Key exists") {
                        skipped_count += 1;
                    } else {
                        failed_keys.push((key, e.to_string()));
                        skipped_count += 1;
                    }

                    if let Some(ref callback) = progress_callback {
                        callback(imported_count + skipped_count, total_keys);
                    }
                }
                Err(e) => {
                    failed_keys.push(("unknown".to_string(), e.to_string()));
                    skipped_count += 1;
                }
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
    if let Some(desktop_dir) = dirs::desktop_dir() {
        desktop_dir.join(default_name)
    } else {
        // Fallback to current directory if desktop path is not available
        let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        current_dir.join(default_name)
    }
}
