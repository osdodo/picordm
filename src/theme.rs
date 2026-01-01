use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThemeColors {
    pub accent: Color,
    pub border_active: Color,
    pub border_default: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_disabled: Color,
    pub text_on_highlight: Color,
    pub text_required: Color,
    pub text_inactive: Color,
    pub text_key_selected: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    pub bg_main: Color,
    pub bg_dialog: Color,
    pub bg_highlight: Color,
    pub info_uptime: Color,
    pub info_clients: Color,
    pub info_keys: Color,
    pub info_memory: Color,
    pub editor_base_bg: Color,
    pub editor_cursor_bg: Color,
}

impl ThemeColors {
    pub fn dark() -> Self {
        Self {
            accent: Color::Rgb(102, 31, 245),
            border_active: Color::Rgb(80, 80, 120),
            border_default: Color::Rgb(80, 80, 80),
            text_primary: Color::White,
            text_secondary: Color::Rgb(187, 187, 187),
            text_disabled: Color::DarkGray,
            text_on_highlight: Color::White,
            text_required: Color::LightRed,
            text_inactive: Color::Rgb(60, 60, 60),
            text_key_selected: Color::Rgb(102, 31, 245),
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::LightRed,
            info: Color::Rgb(101, 79, 246),
            bg_main: Color::Rgb(20, 20, 30),
            bg_dialog: Color::Rgb(25, 25, 35),
            bg_highlight: Color::Rgb(102, 31, 245),
            info_uptime: Color::Magenta,
            info_clients: Color::Rgb(102, 31, 245),
            info_keys: Color::Magenta,
            info_memory: Color::Green,
            editor_base_bg: Color::Rgb(20, 20, 30),
            editor_cursor_bg: Color::White,
        }
    }

    pub fn light() -> Self {
        Self {
            accent: Color::Rgb(102, 31, 245),
            border_active: Color::Rgb(0, 0, 0),
            border_default: Color::Rgb(174, 174, 174),
            text_primary: Color::Rgb(15, 23, 42),
            text_secondary: Color::Rgb(71, 85, 105),
            text_disabled: Color::Rgb(148, 163, 184),
            text_on_highlight: Color::Rgb(255, 255, 255),
            text_required: Color::Rgb(239, 68, 68),
            text_inactive: Color::Rgb(148, 163, 184),
            text_key_selected: Color::Rgb(102, 31, 245),
            success: Color::Rgb(16, 148, 76),
            warning: Color::Rgb(245, 158, 11),
            error: Color::Rgb(239, 68, 68),
            info: Color::Rgb(101, 79, 246),
            bg_main: Color::Rgb(248, 250, 252),
            bg_dialog: Color::Rgb(255, 255, 255),
            bg_highlight: Color::Rgb(99, 102, 241),
            info_uptime: Color::Magenta,
            info_clients: Color::Rgb(102, 31, 245),
            info_keys: Color::Magenta,
            info_memory: Color::Green,
            editor_base_bg: Color::Rgb(248, 250, 252),
            editor_cursor_bg: Color::Rgb(30, 41, 59),
        }
    }

    pub fn from_theme(theme: Theme) -> Self {
        match theme {
            Theme::Dark => Self::dark(),
            Theme::Light => Self::light(),
        }
    }

    pub fn with_transparency(mut self, enable_bg_transparent: bool) -> Self {
        if enable_bg_transparent {
            self.bg_main = Color::Reset;
            self.bg_dialog = Color::Reset;
            self.editor_base_bg = Color::Reset;
        }
        self
    }
}

pub struct ThemeManager {
    current: RwLock<Theme>,
    colors: RwLock<Arc<ThemeColors>>,
    enable_bg_transparent: RwLock<bool>,
}

impl ThemeManager {
    fn new(theme: Theme, enable_bg_transparent: bool) -> Self {
        let colors =
            Arc::new(ThemeColors::from_theme(theme).with_transparency(enable_bg_transparent));
        Self {
            current: RwLock::new(theme),
            colors: RwLock::new(colors),
            enable_bg_transparent: RwLock::new(enable_bg_transparent),
        }
    }

    pub fn get_colors(&self) -> Arc<ThemeColors> {
        self.colors.read().unwrap().clone()
    }

    pub fn get_theme(&self) -> Theme {
        *self.current.read().unwrap()
    }

    pub fn get_enable_bg_transparent(&self) -> bool {
        *self.enable_bg_transparent.read().unwrap()
    }

    pub fn set_theme(&self, theme: Theme) {
        let enable_bg_transparent = *self.enable_bg_transparent.read().unwrap();
        let colors =
            Arc::new(ThemeColors::from_theme(theme).with_transparency(enable_bg_transparent));
        *self.current.write().unwrap() = theme;
        *self.colors.write().unwrap() = colors;
    }

    pub fn set_enable_bg_transparent(&self, enable_bg_transparent: bool) {
        let theme = *self.current.read().unwrap();
        let colors =
            Arc::new(ThemeColors::from_theme(theme).with_transparency(enable_bg_transparent));
        *self.enable_bg_transparent.write().unwrap() = enable_bg_transparent;
        *self.colors.write().unwrap() = colors;
    }
}

static THEME_MANAGER: OnceLock<ThemeManager> = OnceLock::new();

/// Initialize theme manager (should be called once at startup)
pub fn init_theme_manager(theme: Theme, enable_bg_transparent: bool) {
    THEME_MANAGER.get_or_init(|| ThemeManager::new(theme, enable_bg_transparent));
}

fn get_manager() -> &'static ThemeManager {
    THEME_MANAGER
        .get()
        .expect("Theme manager not initialized. Call init_theme_manager() at app startup.")
}

pub fn get_colors() -> Arc<ThemeColors> {
    get_manager().get_colors()
}

pub fn get_theme() -> Theme {
    get_manager().get_theme()
}

pub fn get_enable_bg_transparent() -> bool {
    get_manager().get_enable_bg_transparent()
}

pub fn set_theme(theme: Theme) {
    get_manager().set_theme(theme);
}

pub fn set_enable_bg_transparent(enable_bg_transparent: bool) {
    get_manager().set_enable_bg_transparent(enable_bg_transparent);
}
