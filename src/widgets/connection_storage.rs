use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::models::ConnectionConfig;

pub const APP_NAME: &str = "picordm";
pub const CONFIG_FILE_NAME: &str = "picordm_connections.json";

fn get_config_path() -> Result<PathBuf> {
    // macOS: ~/Library/Application Support/picordm/
    // Linux: ~/.config/picordm/
    // Windows: %APPDATA%\picordm\
    let config_dir = dirs::config_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
        .ok_or_else(|| anyhow::anyhow!("Unable to determine config directory"))?;

    let app_config_dir = config_dir.join(APP_NAME);
    Ok(app_config_dir.join(CONFIG_FILE_NAME))
}

fn ensure_config_dir_exists() -> Result<PathBuf> {
    let config_path = get_config_path()?;
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
    }
    Ok(config_path)
}

pub fn load_connections() -> Vec<ConnectionConfig> {
    let path = match get_config_path() {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };

    if !path.exists() {
        return Vec::new();
    }

    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub fn save_connections(connections: &[ConnectionConfig]) -> Result<()> {
    let path = ensure_config_dir_exists()?;
    let content = serde_json::to_string_pretty(connections)?;
    fs::write(&path, content)
        .with_context(|| format!("Failed to write config file: {:?}", path))?;
    Ok(())
}
