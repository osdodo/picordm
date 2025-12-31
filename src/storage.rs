use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::constants::APP_NAME;

pub struct Storage {
    file_name: String,
}

impl Storage {
    pub fn new(file_name: impl Into<String>) -> Self {
        Self {
            file_name: file_name.into(),
        }
    }

    fn get_config_path(&self) -> Result<PathBuf> {
        // macOS: ~/Library/Application Support/picordm/
        // Linux: ~/.config/picordm/
        // Windows: %APPDATA%\picordm\
        let config_dir = dirs::config_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
            .ok_or_else(|| anyhow::anyhow!("Unable to determine config directory"))?;

        let app_config_dir = config_dir.join(APP_NAME);
        Ok(app_config_dir.join(&self.file_name))
    }

    fn ensure_config_dir_exists(&self) -> Result<PathBuf> {
        let config_path = self.get_config_path()?;
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
        }
        Ok(config_path)
    }

    pub fn load<T>(&self) -> T
    where
        T: for<'de> Deserialize<'de> + Default,
    {
        let path = match self.get_config_path() {
            Ok(p) => p,
            Err(_) => return T::default(),
        };

        if !path.exists() {
            return T::default();
        }

        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => T::default(),
        }
    }

    pub fn save<T>(&self, data: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        let path = self.ensure_config_dir_exists()?;
        let content = serde_json::to_string_pretty(data)?;
        fs::write(&path, content)
            .with_context(|| format!("Failed to write config file: {:?}", path))?;
        Ok(())
    }
}
