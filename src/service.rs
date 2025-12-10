use std::future::Future;
use std::sync::Arc;

use anyhow::Result;
use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands, Client};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct RedisService {
    state: Arc<RwLock<ServiceState>>,
}

struct ServiceState {
    connection: Option<MultiplexedConnection>,
    current_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub uptime_seconds: u64,
    pub connected_clients: u32,
    pub total_keys: u64,
    pub used_memory: u64,
}

#[derive(Debug, Clone)]
pub struct DbInfo {
    pub index: u32,
    pub keys_count: u64,
}

impl RedisService {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(ServiceState {
                connection: None,
                current_url: None,
            })),
        }
    }

    pub async fn connect(&self, url: &str) -> Result<()> {
        let client = Client::open(url)?;
        let connection = client.get_multiplexed_async_connection().await?;
        let mut state = self.state.write().await;
        state.connection = Some(connection);
        state.current_url = Some(url.to_string());
        Ok(())
    }

    pub async fn disconnect(&self) {
        let mut state = self.state.write().await;
        state.connection = None;
        state.current_url = None;
    }

    async fn get_conn(&self) -> Result<MultiplexedConnection> {
        let state = self.state.read().await;
        state
            .connection
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))
    }

    async fn execute_retry<T, F, Fut>(&self, f: F) -> Result<T>
    where
        F: Fn(MultiplexedConnection) -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let conn = self.get_conn().await?;
        match f(conn.clone()).await {
            Ok(v) => Ok(v),
            Err(e) => {
                // Try to downcast to RedisError to check for connection issues
                let should_retry = if let Some(redis_err) = e.downcast_ref::<redis::RedisError>() {
                    redis_err.is_io_error()
                } else {
                    false
                };

                if should_retry {
                    let url = self.state.read().await.current_url.clone();
                    if let Some(url) = url {
                        let _ = self.connect(&url).await;
                        if let Ok(new_conn) = self.get_conn().await {
                            return f(new_conn).await;
                        }
                    }
                }
                Err(e)
            }
        }
    }

    pub async fn get_keys(&self, pattern: &str, db_index: u32) -> Result<Vec<String>> {
        let pattern = pattern.to_string();
        self.execute_retry(move |mut conn| {
            let pattern = pattern.clone();
            async move {
                redis::cmd("SELECT")
                    .arg(db_index)
                    .query_async::<()>(&mut conn)
                    .await?;
                Ok(conn.keys(pattern).await?)
            }
        })
        .await
    }

    pub async fn get_type(&self, key: &str, db_index: u32) -> Result<String> {
        let key = key.to_string();
        self.execute_retry(move |mut conn| {
            let key = key.clone();
            async move {
                redis::cmd("SELECT")
                    .arg(db_index)
                    .query_async::<()>(&mut conn)
                    .await?;
                Ok(redis::cmd("TYPE").arg(key).query_async(&mut conn).await?)
            }
        })
        .await
    }

    pub async fn get_value(&self, key: &str, db_index: u32) -> Result<String> {
        let key_type = self.get_type(key, db_index).await?;
        let key_owned = key.to_string();

        self.execute_retry(move |mut conn| {
            let key = key_owned.clone();
            let key_type = key_type.clone();
            async move {
                redis::cmd("SELECT")
                    .arg(db_index)
                    .query_async::<()>(&mut conn)
                    .await?;
                match key_type.as_str() {
                    "string" => {
                        let bytes: Vec<u8> = conn.get(&key).await?;
                        match String::from_utf8(bytes) {
                            Ok(s) => Ok(s),
                            Err(e) => {
                                let invalid_bytes = e.into_bytes();
                                let hex_string: String = invalid_bytes
                                    .iter()
                                    .take(100)
                                    .map(|b| format!("{:02X}", b))
                                    .collect();

                                let suffix = if invalid_bytes.len() > 100 { "..." } else { "" };
                                Ok(format!(
                                    "<Binary Data: {} bytes>\nHex Preview: {}{}",
                                    invalid_bytes.len(),
                                    hex_string,
                                    suffix
                                ))
                            }
                        }
                    }
                    "list" => {
                        let val: Vec<String> = conn.lrange(&key, 0, -1).await?;
                        Ok(serde_json::to_string_pretty(&val).unwrap_or_default())
                    }
                    "set" => {
                        let val: Vec<String> = conn.smembers(&key).await?;
                        Ok(serde_json::to_string_pretty(&val).unwrap_or_default())
                    }
                    "zset" => {
                        let val: Vec<String> = conn.zrange(&key, 0, -1).await?;
                        Ok(serde_json::to_string_pretty(&val).unwrap_or_default())
                    }
                    "hash" => {
                        let val: std::collections::HashMap<String, String> =
                            conn.hgetall(&key).await?;
                        Ok(serde_json::to_string_pretty(&val).unwrap_or_default())
                    }
                    "none" => Ok("Key does not exist".to_string()),
                    _ => Ok(format!("Unsupported type: {}", key_type)),
                }
            }
        })
        .await
    }

    pub async fn get_server_info(&self) -> Result<(ServerInfo, Vec<DbInfo>)> {
        self.execute_retry(|mut conn| async move {
            let info: String = redis::cmd("INFO")
                .arg("server")
                .arg("clients")
                .arg("keyspace")
                .arg("memory")
                .query_async(&mut conn)
                .await?;

            let mut uptime_seconds = 0;
            let mut connected_clients = 0;
            let mut total_keys = 0;
            let mut used_memory = 0;
            let mut db_list = Vec::new();

            for line in info.lines() {
                let line = line.trim();
                if line.starts_with("uptime_in_seconds:") {
                    if let Some(val) = line.strip_prefix("uptime_in_seconds:") {
                        uptime_seconds = val.trim().parse().unwrap_or(0);
                    }
                } else if line.starts_with("connected_clients:") {
                    if let Some(val) = line.strip_prefix("connected_clients:") {
                        connected_clients = val.trim().parse().unwrap_or(0);
                    }
                } else if line.starts_with("used_memory:") {
                    if let Some(val) = line.strip_prefix("used_memory:") {
                        used_memory = val.trim().parse().unwrap_or(0);
                    }
                } else if line.starts_with("db") {
                    // Parse keyspace line like: db0:keys=123,expires=0,avg_ttl=0
                    if let Some(colon_pos) = line.find(':') {
                        let db_name = &line[..colon_pos];
                        if let Some(db_num_str) = db_name.strip_prefix("db") {
                            if let Ok(db_num) = db_num_str.parse::<u32>() {
                                let mut keys_count = 0;
                                if let Some(keys_part) = line.split("keys=").nth(1) {
                                    if let Some(keys_str) = keys_part.split(',').next() {
                                        keys_count = keys_str.parse().unwrap_or(0);
                                        total_keys += keys_count;
                                    }
                                }
                                db_list.push(DbInfo {
                                    index: db_num,
                                    keys_count,
                                });
                            }
                        }
                    }
                }
            }

            // Ensure at least db0 exists
            if db_list.is_empty() {
                db_list.push(DbInfo {
                    index: 0,
                    keys_count: 0,
                });
            }

            db_list.sort_by_key(|db| db.index);

            Ok((
                ServerInfo {
                    uptime_seconds,
                    connected_clients,
                    total_keys,
                    used_memory,
                },
                db_list,
            ))
        })
        .await
    }

    pub async fn select_db(&self, db_index: u32) -> Result<()> {
        self.execute_retry(move |mut conn| async move {
            redis::cmd("SELECT")
                .arg(db_index)
                .query_async::<()>(&mut conn)
                .await?;
            Ok(())
        })
        .await
    }

    pub async fn set_value(&self, key: &str, value: &str, db_index: u32) -> Result<()> {
        let key = key.to_string();
        let value = value.to_string();
        self.execute_retry(move |mut conn| {
            let key = key.clone();
            let value = value.clone();
            async move {
                redis::cmd("SELECT")
                    .arg(db_index)
                    .query_async::<()>(&mut conn)
                    .await?;
                let _: () = conn.set(&key, &value).await?;
                Ok(())
            }
        })
        .await
    }

    pub async fn delete_key(&self, key: &str, db_index: u32) -> Result<()> {
        let key = key.to_string();
        self.execute_retry(move |mut conn| {
            let key = key.clone();
            async move {
                redis::cmd("SELECT")
                    .arg(db_index)
                    .query_async::<()>(&mut conn)
                    .await?;
                let _: u32 = conn.del(&key).await?;
                Ok(())
            }
        })
        .await
    }

    pub async fn execute_command(&self, args: &[&str], db_index: u32) -> Result<String> {
        if args.is_empty() {
            return Ok(String::new());
        }

        let args_owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        
        self.execute_retry(move |mut conn| {
            let args = args_owned.clone();
            async move {
                // Select the correct database first
                redis::cmd("SELECT")
                    .arg(db_index)
                    .query_async::<()>(&mut conn)
                    .await?;

                let mut cmd = redis::cmd(&args[0]);
                for arg in &args[1..] {
                    cmd.arg(arg);
                }

                let result: redis::RedisResult<redis::Value> = cmd.query_async(&mut conn).await;
                
                match result {
                    Ok(value) => Ok(format_redis_value(&value)),
                    Err(e) => Err(anyhow::anyhow!("{}", e)),
                }
            }
        })
        .await
    }
}

fn format_redis_value(value: &redis::Value) -> String {
    match value {
        redis::Value::Nil => "(nil)".to_string(),
        redis::Value::Int(i) => format!("(integer) {}", i),
        redis::Value::BulkString(bytes) => {
            String::from_utf8(bytes.clone()).unwrap_or_else(|_| format!("{:?}", bytes))
        }
        redis::Value::Array(arr) => {
            if arr.is_empty() {
                "(empty array)".to_string()
            } else {
                let mut result = String::new();
                for (idx, item) in arr.iter().enumerate() {
                    result.push_str(&format!("{}. {}\n", idx + 1, format_redis_value(item)));
                }
                result
            }
        }
        redis::Value::SimpleString(s) => s.clone(),
        redis::Value::Okay => "OK".to_string(),
        redis::Value::Map(map) => {
            let mut result = String::new();
            for (key, val) in map {
                result.push_str(&format!("{}: {}\n", format_redis_value(key), format_redis_value(val)));
            }
            result
        }
        redis::Value::Attribute { data, attributes } => {
            format!("Data: {}\nAttributes: {}", 
                format_redis_value(data), 
                format_redis_value(&redis::Value::Map(attributes.clone())))
        }
        redis::Value::Set(set) => {
            let mut result = String::new();
            for (idx, item) in set.iter().enumerate() {
                result.push_str(&format!("{}. {}\n", idx + 1, format_redis_value(item)));
            }
            result
        }
        redis::Value::Double(d) => format!("(double) {}", d),
        redis::Value::Boolean(b) => format!("(boolean) {}", b),
        redis::Value::VerbatimString { format, text } => {
            format!("[{:?}] {}", format, text)
        }
        redis::Value::BigNumber(bn) => format!("(big number) {:?}", bn),
        redis::Value::Push { kind, data } => {
            let mut result = format!("Push [{:?}]:\n", kind);
            for item in data {
                result.push_str(&format!("{}\n", format_redis_value(item)));
            }
            result
        }
        redis::Value::ServerError(e) => format!("Server Error: {:?}", e),
    }
}
