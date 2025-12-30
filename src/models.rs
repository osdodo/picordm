use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Screen {
    Connection,
    Dashboard,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    KeyList,
    KeyContent,
    CommandMode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub password: Option<String>,
    pub username: Option<String>,
    #[serde(default)]
    pub db_aliases: HashMap<u32, String>,
    #[serde(default)]
    pub use_tls: bool,
    #[serde(default)]
    pub allow_insecure_tls: bool,
    pub sni: Option<String>,
    #[serde(default)]
    pub is_cluster: bool,
    #[serde(default)]
    pub cluster_nodes: Vec<String>,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Localhost".to_string(),
            host: "127.0.0.1".to_string(),
            port: 6379,
            password: None,
            username: None,
            db_aliases: HashMap::new(),
            use_tls: false,
            allow_insecure_tls: false,
            sni: None,
            is_cluster: false,
            cluster_nodes: Vec::new(),
        }
    }
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
