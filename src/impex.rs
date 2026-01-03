use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
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
    // Check file size
    const WARN_SIZE_MB: u64 = 200;
    const STREAM_THRESHOLD_MB: u64 = 10;

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

    // For large files, use streaming parsing; for small files, use the traditional method (which is faster).
    if file_size_mb > STREAM_THRESHOLD_MB {
        import_redis_data_streaming(file_path, database, overwrite, progress_callback).await
    } else {
        import_redis_data_traditional(file_path, database, overwrite, progress_callback).await
    }
}

async fn import_redis_data_traditional<F>(
    file_path: &Path,
    database: u32,
    overwrite: bool,
    progress_callback: Option<F>,
) -> Result<ImportResult>
where
    F: Fn(usize, usize) + Send + Sync,
{
    let file = File::open(file_path)
        .with_context(|| format!("Failed to open Redis import file: {:?}", file_path))?;
    let reader = BufReader::new(file);

    let import_data: RedisImportData =
        serde_json::from_reader(reader).context("Failed to parse JSON from Redis import file")?;

    let mut imported_count = 0;
    let mut skipped_count = 0;
    let mut failed_keys = Vec::new();

    let target_database = import_data.database.unwrap_or(database);

    const BATCH_SIZE: usize = 10;
    let keys_vec: Vec<_> = import_data.keys.into_iter().collect();
    let total_keys = keys_vec.len();

    if let Some(ref callback) = progress_callback {
        callback(0, total_keys);
    }

    for chunk in keys_vec.chunks(BATCH_SIZE) {
        let mut tasks = Vec::new();

        for (key, key_data) in chunk {
            let key_clone = key.clone();
            let key_data_clone = key_data.clone();

            tasks.push(tokio::spawn(async move {
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

async fn import_redis_data_streaming<F>(
    file_path: &Path,
    database: u32,
    overwrite: bool,
    progress_callback: Option<F>,
) -> Result<ImportResult>
where
    F: Fn(usize, usize) + Send + Sync,
{
    let file = File::open(file_path)
        .with_context(|| format!("Failed to open Redis import file: {:?}", file_path))?;
    let reader = BufReader::new(file);

    let mut imported_count = 0;
    let mut skipped_count = 0;
    let mut failed_keys = Vec::new();
    let mut target_database = database;

    // First, scan the files to obtain the total number of keys and database settings.
    let (total_keys, file_database) = count_keys_in_file(file_path)?;
    if let Some(db) = file_database {
        target_database = db;
    }

    if let Some(ref callback) = progress_callback {
        callback(0, total_keys);
    }

    // Streaming JSON parsing
    let mut in_keys_section = false;
    let mut current_key: Option<String> = None;
    let mut current_value_lines: Vec<String> = Vec::new();
    let mut pending_keys: Vec<(String, RedisKeyData)> = Vec::new();

    const BATCH_SIZE: usize = 10;

    for line_result in reader.lines() {
        let line = line_result.context("Failed to read line")?;
        let trimmed = line.trim();

        // Detecting entry into the keys section
        if !in_keys_section {
            if trimmed.starts_with("\"keys\"") && trimmed.contains('{') {
                in_keys_section = true;
            }
            continue;
        }

        // The key detection section ends: only } or },
        if (trimmed == "}" || trimmed == "},") && current_key.is_none() {
            break;
        }

        // Parse key-value pairs
        if current_key.is_none() {
            // Try parsing the new key
            if let Some(key) = extract_key_name(trimmed) {
                current_key = Some(key);
                // Check if there are complete values ​​in the same row.
                if let Some(start) = trimmed.find(": {") {
                    let value_start = start + 2;
                    current_value_lines.push(trimmed[value_start..].to_string());
                }
            }
        } else {
            current_value_lines.push(line.clone());

            // Check if the value is complete (by detecting closed curly braces).
            let combined = current_value_lines.join("\n");
            if is_json_complete(&combined) {
                if let Some(key) = current_key.take() {
                    // Parse key data
                    let clean_value = combined.trim().trim_end_matches(',');
                    match serde_json::from_str::<RedisKeyData>(clean_value) {
                        Ok(key_data) => {
                            pending_keys.push((key, key_data));
                        }
                        Err(e) => {
                            failed_keys.push((key, format!("Parse error: {}", e)));
                            skipped_count += 1;
                        }
                    }
                }
                current_value_lines.clear();

                // Batch processing
                if pending_keys.len() >= BATCH_SIZE {
                    let (imp, skip, failed) = process_key_batch(
                        std::mem::take(&mut pending_keys),
                        target_database,
                        overwrite,
                    )
                    .await;
                    imported_count += imp;
                    skipped_count += skip;
                    failed_keys.extend(failed);

                    if let Some(ref callback) = progress_callback {
                        callback(imported_count + skipped_count, total_keys);
                    }
                }
            }
        }
    }

    // Process the remaining keys
    if !pending_keys.is_empty() {
        let (imp, skip, failed) = process_key_batch(pending_keys, target_database, overwrite).await;
        imported_count += imp;
        skipped_count += skip;
        failed_keys.extend(failed);

        if let Some(ref callback) = progress_callback {
            callback(imported_count + skipped_count, total_keys);
        }
    }

    Ok(ImportResult {
        imported_count,
        skipped_count,
        failed_keys,
    })
}

/// Scan files to get the total number of keys and database settings
fn count_keys_in_file(file_path: &Path) -> Result<(usize, Option<u32>)> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut count = 0;
    let mut database = None;
    let mut in_keys_section = false;
    let mut in_key_value = false;
    let mut value_lines: Vec<String> = Vec::new();

    for line_result in reader.lines() {
        let line = line_result?;
        let trimmed = line.trim();

        // Parse database fields (before keys section)
        if !in_keys_section
            && trimmed.starts_with("\"database\"")
            && let Some(db) = extract_database_value(trimmed)
        {
            database = Some(db);
            continue;
        }

        // Detecting entry into the keys section
        if !in_keys_section {
            if trimmed.starts_with("\"keys\"") && trimmed.contains('{') {
                in_keys_section = true;
            }
            continue;
        }

        // Collecting the value of a certain key
        if in_key_value {
            value_lines.push(line.clone());
            let combined = value_lines.join("\n");
            if is_json_complete(&combined) {
                // The value is complete; increment the count by 1.
                count += 1;
                in_key_value = false;
                value_lines.clear();
            }
            continue;
        }

        // The check indicates the end of the keys section (indentation level returns to the closing brackets of keys).
        if trimmed == "}" || trimmed == "}," {
            break;
        }

        // Start of detecting new key
        if let Some(_key) = extract_key_name(trimmed) {
            in_key_value = true;
            // Extract the beginning part of value
            if let Some(start) = trimmed.find(": {") {
                let value_start = start + 2;
                value_lines.push(trimmed[value_start..].to_string());
                // Checking if they are on the same line completes the task.
                let combined = value_lines.join("\n");
                if is_json_complete(&combined) {
                    count += 1;
                    in_key_value = false;
                    value_lines.clear();
                }
            }
        }
    }

    Ok((count, database))
}

/// Extract key name from row
fn extract_key_name(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with('"') {
        return None;
    }

    // Find the end of the key name
    let rest = &trimmed[1..];
    let end_quote = rest.find('"')?;
    let key = &rest[..end_quote];

    // Make sure it is followed by ": {"
    let after_key = &rest[end_quote + 1..].trim_start();
    if after_key.starts_with(": {") {
        Some(key.to_string())
    } else {
        None
    }
}

/// Extract database values ​​from rows
fn extract_database_value(line: &str) -> Option<u32> {
    // 格式: "database": 0,
    let parts: Vec<&str> = line.split(':').collect();
    if parts.len() >= 2 {
        let value_part = parts[1].trim().trim_end_matches(',');
        value_part.parse().ok()
    } else {
        None
    }
}

/// Check if the JSON string is complete (curly braces match)
fn is_json_complete(s: &str) -> bool {
    let mut depth = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for ch in s.chars() {
        if escape_next {
            escape_next = false;
            continue;
        }

        match ch {
            '\\' if in_string => escape_next = true,
            '"' => in_string = !in_string,
            '{' if !in_string => depth += 1,
            '}' if !in_string => depth -= 1,
            _ => {}
        }
    }

    depth == 0 && s.trim().starts_with('{')
}

/// Batch processing keys
async fn process_key_batch(
    keys: Vec<(String, RedisKeyData)>,
    database: u32,
    overwrite: bool,
) -> (usize, usize, Vec<(String, String)>) {
    let mut imported = 0;
    let mut skipped = 0;
    let mut failed = Vec::new();

    let mut tasks = Vec::new();

    for (key, key_data) in keys {
        let key_clone = key.clone();
        let key_data_clone = key_data.clone();

        tasks.push(tokio::spawn(async move {
            let exists = if !overwrite {
                get_redis_service()
                    .key_exists(&key_clone, database)
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

            let result = import_redis_key(&key_clone, &key_data_clone, database).await;

            if result.is_ok()
                && let Some(ttl) = key_data_clone.ttl
                && ttl > 0
            {
                let _ = get_redis_service()
                    .set_key_ttl(&key_clone, ttl, database)
                    .await;
            }

            (key_clone, result)
        }));
    }

    for task in tasks {
        match task.await {
            Ok((_, Ok(_))) => imported += 1,
            Ok((key, Err(e))) => {
                if e.to_string().contains("Key exists") {
                    skipped += 1;
                } else {
                    failed.push((key, e.to_string()));
                    skipped += 1;
                }
            }
            Err(e) => {
                failed.push(("unknown".to_string(), e.to_string()));
                skipped += 1;
            }
        }
    }

    (imported, skipped, failed)
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
