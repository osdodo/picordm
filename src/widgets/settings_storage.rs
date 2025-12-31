use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::constants::SETTINGS_FILE_NAME;
use crate::storage::Storage;
use crate::theme::Theme;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct AppSettings {
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub enable_bg_transparent: bool,
}

pub fn load_settings() -> AppSettings {
    let storage = Storage::new(SETTINGS_FILE_NAME);
    storage.load()
}

pub fn save_settings(settings: &AppSettings) -> Result<()> {
    let storage = Storage::new(SETTINGS_FILE_NAME);
    storage.save(settings)
}
