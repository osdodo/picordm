use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::constants::SETTINGS_FILE_NAME;
use crate::storage::Storage;
use crate::theme::Theme;

#[derive(Debug, Serialize, Deserialize, Default)]
struct AppSettings {
    #[serde(default)]
    theme: Theme,
}

pub fn load_theme() -> Theme {
    let storage = Storage::new(SETTINGS_FILE_NAME);
    let settings: AppSettings = storage.load();
    settings.theme
}

pub fn save_theme(theme: Theme) -> Result<()> {
    let storage = Storage::new(SETTINGS_FILE_NAME);
    let settings = AppSettings { theme };
    storage.save(&settings)
}
