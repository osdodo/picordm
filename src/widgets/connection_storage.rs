use anyhow::Result;

use crate::constants::CONNECTIONS_FILE_NAME;
use crate::models::ConnectionConfig;
use crate::storage::Storage;

pub fn load_connections() -> Vec<ConnectionConfig> {
    let storage = Storage::new(CONNECTIONS_FILE_NAME);
    storage.load()
}

pub fn save_connections(connections: &[ConnectionConfig]) -> Result<()> {
    let storage = Storage::new(CONNECTIONS_FILE_NAME);
    storage.save(connections)
}
