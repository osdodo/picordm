use std::future::Future;
use std::sync::Arc;

use anyhow::Result;
use redis::aio::MultiplexedConnection;
use redis::cluster_async::ClusterConnection;
use redis::{AsyncCommands, Client, cluster::ClusterClient};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct RedisService {
    state: Arc<RwLock<ServiceState>>,
}

enum ServiceState {
    Standalone {
        connection: Option<MultiplexedConnection>,
        current_url: Option<String>,
    },
    Cluster {
        connection: Option<ClusterConnection>,
        current_nodes: Option<Vec<String>>,
    },
    Disconnected,
}

#[derive(Clone)]
enum ConnectionType {
    Standalone(MultiplexedConnection),
    Cluster(ClusterConnection),
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
            state: Arc::new(RwLock::new(ServiceState::Disconnected)),
        }
    }

    pub async fn connect(&self, url: &str) -> Result<()> {
        let client = Client::open(url)?;
        let connection = client.get_multiplexed_async_connection().await?;
        let mut state = self.state.write().await;
        *state = ServiceState::Standalone {
            connection: Some(connection),
            current_url: Some(url.to_string()),
        };
        Ok(())
    }

    pub async fn connect_cluster(&self, nodes: Vec<String>) -> Result<()> {
        let client = ClusterClient::new(nodes.clone())?;
        let connection = client.get_async_connection().await?;
        let mut state = self.state.write().await;
        *state = ServiceState::Cluster {
            connection: Some(connection),
            current_nodes: Some(nodes),
        };
        Ok(())
    }

    pub async fn disconnect(&self) {
        let mut state = self.state.write().await;
        *state = ServiceState::Disconnected;
    }

    async fn get_conn(&self) -> Result<ConnectionType> {
        let state = self.state.read().await;
        match &*state {
            ServiceState::Standalone { connection, .. } => Ok(ConnectionType::Standalone(
                connection
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("Not connected"))?,
            )),
            ServiceState::Cluster { connection, .. } => Ok(ConnectionType::Cluster(
                connection
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("Not connected"))?,
            )),
            ServiceState::Disconnected => Err(anyhow::anyhow!("Not connected")),
        }
    }

    async fn execute_retry<T, F, Fut>(&self, f: F) -> Result<T>
    where
        F: Fn(ConnectionType) -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let conn = self.get_conn().await?;
        match f(conn.clone()).await {
            Ok(v) => Ok(v),
            Err(e) => {
                let should_retry = if let Some(redis_err) = e.downcast_ref::<redis::RedisError>() {
                    redis_err.is_io_error()
                } else {
                    false
                };

                if should_retry {
                    let state = self.state.read().await;
                    match &*state {
                        ServiceState::Standalone { current_url, .. } => {
                            if let Some(url) = current_url {
                                let _ = self.connect(url).await;
                                if let Ok(new_conn) = self.get_conn().await {
                                    return f(new_conn).await;
                                }
                            }
                        }
                        ServiceState::Cluster { current_nodes, .. } => {
                            if let Some(nodes) = current_nodes {
                                let _ = self.connect_cluster(nodes.clone()).await;
                                if let Ok(new_conn) = self.get_conn().await {
                                    return f(new_conn).await;
                                }
                            }
                        }
                        ServiceState::Disconnected => {}
                    }
                }
                Err(e)
            }
        }
    }

    // Execute a SELECT statement in Standalone mode, and then perform the operation.
    async fn with_db<T, F, Fut>(&self, db_index: u32, f: F) -> Result<T>
    where
        F: Fn(ConnectionType) -> Fut,
        Fut: Future<Output = Result<T>>,
        T: Send + 'static,
    {
        self.execute_retry(|conn| async {
            match conn {
                ConnectionType::Standalone(mut c) => {
                    redis::cmd("SELECT")
                        .arg(db_index)
                        .query_async::<()>(&mut c)
                        .await?;
                    f(ConnectionType::Standalone(c)).await
                }
                ConnectionType::Cluster(c) => f(ConnectionType::Cluster(c)).await,
            }
        })
        .await
    }

    pub async fn get_keys(&self, pattern: &str, db_index: u32) -> Result<Vec<String>> {
        let pattern = pattern.to_string();
        self.with_db(db_index, |conn| {
            let pattern = pattern.clone();
            async move {
                match conn {
                    ConnectionType::Standalone(mut c) => Ok(c.keys(pattern).await?),
                    ConnectionType::Cluster(mut c) => Ok(c.keys(pattern).await?),
                }
            }
        })
        .await
    }

    pub async fn get_type(&self, key: &str, db_index: u32) -> Result<String> {
        let key = key.to_string();
        self.with_db(db_index, |conn| {
            let key = key.clone();
            async move {
                match conn {
                    ConnectionType::Standalone(mut c) => {
                        Ok(redis::cmd("TYPE").arg(key).query_async(&mut c).await?)
                    }
                    ConnectionType::Cluster(mut c) => {
                        Ok(redis::cmd("TYPE").arg(key).query_async(&mut c).await?)
                    }
                }
            }
        })
        .await
    }

    pub async fn get_value(&self, key: &str, db_index: u32) -> Result<String> {
        let key_type = self.get_type(key, db_index).await?;
        let key_owned = key.to_string();

        self.with_db(db_index, |conn| {
            let key = key_owned.clone();
            let key_type = key_type.clone();
            async move {
                match conn {
                    ConnectionType::Standalone(mut c) => {
                        get_value_by_type(&mut c, &key, &key_type).await
                    }
                    ConnectionType::Cluster(mut c) => {
                        get_value_by_type(&mut c, &key, &key_type).await
                    }
                }
            }
        })
        .await
    }

    pub async fn get_server_info(&self) -> Result<(ServerInfo, Vec<DbInfo>)> {
        self.execute_retry(|conn| async move {
            match conn {
                ConnectionType::Standalone(mut c) => {
                    let info: String = redis::cmd("INFO")
                        .arg("server")
                        .arg("clients")
                        .arg("keyspace")
                        .arg("memory")
                        .query_async(&mut c)
                        .await?;
                    parse_server_info(&info)
                }
                ConnectionType::Cluster(mut c) => {
                    // For cluster mode, INFO returns a Map with node addresses as keys
                    // We need to extract the first node's info or aggregate the data
                    let result: redis::Value = redis::cmd("INFO")
                        .arg("server")
                        .arg("clients")
                        .arg("keyspace")
                        .arg("memory")
                        .query_async(&mut c)
                        .await?;

                    let info_str = match result {
                        redis::Value::BulkString(bytes) => String::from_utf8(bytes)
                            .map_err(|e| anyhow::anyhow!("Failed to parse INFO response: {}", e))?,
                        redis::Value::SimpleString(s) => s,
                        redis::Value::VerbatimString { text, .. } => text,
                        redis::Value::Map(map) => {
                            // Cluster mode returns a map of node -> info
                            // Extract the first node's info
                            if let Some((_, value)) = map.first() {
                                match value {
                                    redis::Value::BulkString(bytes) => {
                                        String::from_utf8(bytes.clone()).map_err(|e| {
                                            anyhow::anyhow!("Failed to parse INFO response: {}", e)
                                        })?
                                    }
                                    redis::Value::SimpleString(s) => s.clone(),
                                    redis::Value::VerbatimString { text, .. } => text.clone(),
                                    _ => {
                                        return Err(anyhow::anyhow!(
                                            "Unexpected INFO value type in map: {:?}",
                                            value
                                        ));
                                    }
                                }
                            } else {
                                return Err(anyhow::anyhow!(
                                    "Empty INFO response map from cluster"
                                ));
                            }
                        }
                        _ => {
                            return Err(anyhow::anyhow!(
                                "Unexpected INFO response type: {:?}",
                                result
                            ));
                        }
                    };

                    parse_server_info(&info_str)
                }
            }
        })
        .await
    }

    pub async fn select_db(&self, db_index: u32) -> Result<()> {
        self.execute_retry(move |conn| async move {
            match conn {
                ConnectionType::Standalone(mut c) => {
                    redis::cmd("SELECT")
                        .arg(db_index)
                        .query_async::<()>(&mut c)
                        .await?;
                    Ok(())
                }
                ConnectionType::Cluster(_) => {
                    // Cluster mode doesn't support SELECT
                    Ok(())
                }
            }
        })
        .await
    }

    pub async fn set_value(&self, key: &str, value: &str, db_index: u32) -> Result<()> {
        let key = key.to_string();
        let value = value.to_string();
        self.with_db(db_index, |conn| {
            let key = key.clone();
            let value = value.clone();
            async move {
                match conn {
                    ConnectionType::Standalone(mut c) => {
                        let _: () = c.set(&key, &value).await?;
                        Ok(())
                    }
                    ConnectionType::Cluster(mut c) => {
                        let _: () = c.set(&key, &value).await?;
                        Ok(())
                    }
                }
            }
        })
        .await
    }

    pub async fn delete_key(&self, key: &str, db_index: u32) -> Result<()> {
        let key = key.to_string();
        self.with_db(db_index, |conn| {
            let key = key.clone();
            async move {
                match conn {
                    ConnectionType::Standalone(mut c) => {
                        let _: u32 = c.del(&key).await?;
                        Ok(())
                    }
                    ConnectionType::Cluster(mut c) => {
                        let _: u32 = c.del(&key).await?;
                        Ok(())
                    }
                }
            }
        })
        .await
    }

    pub async fn execute_command(&self, args: &[&str], db_index: u32) -> Result<String> {
        if args.is_empty() {
            return Ok(String::new());
        }

        let args_owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();

        self.with_db(db_index, |conn| {
            let args = args_owned.clone();
            async move {
                let mut cmd = redis::cmd(&args[0]);
                for arg in &args[1..] {
                    cmd.arg(arg);
                }

                let result: redis::RedisResult<redis::Value> = match conn {
                    ConnectionType::Standalone(mut c) => cmd.query_async(&mut c).await,
                    ConnectionType::Cluster(mut c) => cmd.query_async(&mut c).await,
                };

                match result {
                    Ok(value) => Ok(format_redis_value(&value)),
                    Err(e) => Err(anyhow::anyhow!("{}", e)),
                }
            }
        })
        .await
    }

    pub async fn get_key_ttl(&self, key: &str, db_index: u32) -> Result<i64> {
        let key = key.to_string();
        self.with_db(db_index, |conn| {
            let key = key.clone();
            async move {
                match conn {
                    ConnectionType::Standalone(mut c) => {
                        Ok(redis::cmd("TTL").arg(key).query_async(&mut c).await?)
                    }
                    ConnectionType::Cluster(mut c) => {
                        Ok(redis::cmd("TTL").arg(key).query_async(&mut c).await?)
                    }
                }
            }
        })
        .await
    }

    pub async fn key_exists(&self, key: &str, db_index: u32) -> Result<bool> {
        let key = key.to_string();
        self.with_db(db_index, |conn| {
            let key = key.clone();
            async move {
                let exists: i32 = match conn {
                    ConnectionType::Standalone(mut c) => {
                        redis::cmd("EXISTS").arg(key).query_async(&mut c).await?
                    }
                    ConnectionType::Cluster(mut c) => {
                        redis::cmd("EXISTS").arg(key).query_async(&mut c).await?
                    }
                };
                Ok(exists > 0)
            }
        })
        .await
    }

    pub async fn set_key_ttl(&self, key: &str, ttl: i64, db_index: u32) -> Result<()> {
        let key = key.to_string();
        self.with_db(db_index, |conn| {
            let key = key.clone();
            async move {
                match conn {
                    ConnectionType::Standalone(mut c) => {
                        let _: () = redis::cmd("EXPIRE")
                            .arg(key)
                            .arg(ttl)
                            .query_async(&mut c)
                            .await?;
                        Ok(())
                    }
                    ConnectionType::Cluster(mut c) => {
                        let _: () = redis::cmd("EXPIRE")
                            .arg(key)
                            .arg(ttl)
                            .query_async(&mut c)
                            .await?;
                        Ok(())
                    }
                }
            }
        })
        .await
    }

    // Type-specific getters for export
    pub async fn get_list_values(&self, key: &str, db_index: u32) -> Result<Vec<String>> {
        let key = key.to_string();
        self.with_db(db_index, |conn| {
            let key = key.clone();
            async move {
                match conn {
                    ConnectionType::Standalone(mut c) => Ok(c.lrange(&key, 0, -1).await?),
                    ConnectionType::Cluster(mut c) => Ok(c.lrange(&key, 0, -1).await?),
                }
            }
        })
        .await
    }

    pub async fn get_set_values(&self, key: &str, db_index: u32) -> Result<Vec<String>> {
        let key = key.to_string();
        self.with_db(db_index, |conn| {
            let key = key.clone();
            async move {
                match conn {
                    ConnectionType::Standalone(mut c) => Ok(c.smembers(&key).await?),
                    ConnectionType::Cluster(mut c) => Ok(c.smembers(&key).await?),
                }
            }
        })
        .await
    }

    pub async fn get_zset_values(&self, key: &str, db_index: u32) -> Result<Vec<(String, f64)>> {
        let key = key.to_string();
        self.with_db(db_index, |conn| {
            let key = key.clone();
            async move {
                match conn {
                    ConnectionType::Standalone(mut c) => {
                        Ok(c.zrange_withscores(&key, 0, -1).await?)
                    }
                    ConnectionType::Cluster(mut c) => Ok(c.zrange_withscores(&key, 0, -1).await?),
                }
            }
        })
        .await
    }

    pub async fn get_hash_values(
        &self,
        key: &str,
        db_index: u32,
    ) -> Result<std::collections::HashMap<String, String>> {
        let key = key.to_string();
        self.with_db(db_index, |conn| {
            let key = key.clone();
            async move {
                match conn {
                    ConnectionType::Standalone(mut c) => Ok(c.hgetall(&key).await?),
                    ConnectionType::Cluster(mut c) => Ok(c.hgetall(&key).await?),
                }
            }
        })
        .await
    }

    // Type-specific setters for import
    pub async fn set_list_values(&self, key: &str, values: &[String], db_index: u32) -> Result<()> {
        let key = key.to_string();
        let values = values.to_vec();
        self.with_db(db_index, |conn| {
            let key = key.clone();
            let values = values.clone();
            async move {
                match conn {
                    ConnectionType::Standalone(mut c) => {
                        let _: () = c.del(&key).await?;
                        if !values.is_empty() {
                            let _: usize = c.lpush(&key, &values).await?;
                        }
                        Ok(())
                    }
                    ConnectionType::Cluster(mut c) => {
                        let _: () = c.del(&key).await?;
                        if !values.is_empty() {
                            let _: usize = c.lpush(&key, &values).await?;
                        }
                        Ok(())
                    }
                }
            }
        })
        .await
    }

    pub async fn set_set_values(&self, key: &str, values: &[String], db_index: u32) -> Result<()> {
        let key = key.to_string();
        let values = values.to_vec();
        self.with_db(db_index, |conn| {
            let key = key.clone();
            let values = values.clone();
            async move {
                match conn {
                    ConnectionType::Standalone(mut c) => {
                        let _: () = c.del(&key).await?;
                        if !values.is_empty() {
                            let _: usize = c.sadd(&key, &values).await?;
                        }
                        Ok(())
                    }
                    ConnectionType::Cluster(mut c) => {
                        let _: () = c.del(&key).await?;
                        if !values.is_empty() {
                            let _: usize = c.sadd(&key, &values).await?;
                        }
                        Ok(())
                    }
                }
            }
        })
        .await
    }

    pub async fn set_zset_values(
        &self,
        key: &str,
        values: &[(String, f64)],
        db_index: u32,
    ) -> Result<()> {
        let key = key.to_string();
        let values = values.to_vec();
        self.with_db(db_index, |conn| {
            let key = key.clone();
            let values = values.clone();
            async move {
                match conn {
                    ConnectionType::Standalone(mut c) => {
                        let _: () = c.del(&key).await?;
                        for (member, score) in &values {
                            let _: usize = c.zadd(&key, member, *score).await?;
                        }
                        Ok(())
                    }
                    ConnectionType::Cluster(mut c) => {
                        let _: () = c.del(&key).await?;
                        for (member, score) in &values {
                            let _: usize = c.zadd(&key, member, *score).await?;
                        }
                        Ok(())
                    }
                }
            }
        })
        .await
    }

    pub async fn set_hash_values(
        &self,
        key: &str,
        values: &[(String, String)],
        db_index: u32,
    ) -> Result<()> {
        let key = key.to_string();
        let values = values.to_vec();
        self.with_db(db_index, |conn| {
            let key = key.clone();
            let values = values.clone();
            async move {
                match conn {
                    ConnectionType::Standalone(mut c) => {
                        let _: () = c.del(&key).await?;
                        if !values.is_empty() {
                            let _: () = c.hset_multiple(&key, &values).await?;
                        }
                        Ok(())
                    }
                    ConnectionType::Cluster(mut c) => {
                        let _: () = c.del(&key).await?;
                        if !values.is_empty() {
                            let _: () = c.hset_multiple(&key, &values).await?;
                        }
                        Ok(())
                    }
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
                result.push_str(&format!(
                    "{}: {}\n",
                    format_redis_value(key),
                    format_redis_value(val)
                ));
            }
            result
        }
        redis::Value::Attribute { data, attributes } => {
            format!(
                "Data: {}\nAttributes: {}",
                format_redis_value(data),
                format_redis_value(&redis::Value::Map(attributes.clone()))
            )
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

async fn get_value_by_type<C>(conn: &mut C, key: &str, key_type: &str) -> Result<String>
where
    C: AsyncCommands,
{
    match key_type {
        "string" => {
            let bytes: Vec<u8> = conn.get(key).await?;
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
            let val: Vec<String> = conn.lrange(key, 0, -1).await?;
            Ok(serde_json::to_string_pretty(&val).unwrap_or_default())
        }
        "set" => {
            let val: Vec<String> = conn.smembers(key).await?;
            Ok(serde_json::to_string_pretty(&val).unwrap_or_default())
        }
        "zset" => {
            let val: Vec<String> = conn.zrange(key, 0, -1).await?;
            Ok(serde_json::to_string_pretty(&val).unwrap_or_default())
        }
        "hash" => {
            let val: std::collections::HashMap<String, String> = conn.hgetall(key).await?;
            Ok(serde_json::to_string_pretty(&val).unwrap_or_default())
        }
        "none" => Ok("Key does not exist".to_string()),
        _ => Ok(format!("Unsupported type: {}", key_type)),
    }
}

fn parse_server_info(info: &str) -> Result<(ServerInfo, Vec<DbInfo>)> {
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
        } else if line.starts_with("db")
            && let Some(colon_pos) = line.find(':')
        {
            let db_name = &line[..colon_pos];
            if let Some(db_num_str) = db_name.strip_prefix("db")
                && let Ok(db_num) = db_num_str.parse::<u32>()
            {
                let mut keys_count = 0;
                if let Some(keys_part) = line.split("keys=").nth(1)
                    && let Some(keys_str) = keys_part.split(',').next()
                {
                    keys_count = keys_str.parse().unwrap_or(0);
                    total_keys += keys_count;
                }
                db_list.push(DbInfo {
                    index: db_num,
                    keys_count,
                });
            }
        }
    }

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
}
