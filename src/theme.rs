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
    pub active_border: Color,
    pub inactive_border: Color,
    pub required: Color,
    pub highlight_bg: Color,
    pub accent: Color,

    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_disabled: Color,
    pub text_on_highlight: Color,

    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub error_dark: Color,
    pub info: Color,

    pub cyan: Color,

    pub bg_main: Color,
    pub bg_dialog: Color,
    pub bg_list_item: Color,
    pub bg_highlight: Color,
    pub bg_selected: Color,
    pub bg_error: Color,

    pub directory: Color,
    pub file: Color,
    pub key: Color,
    pub inactive_text: Color,

    pub editor_base_bg: Color,
    pub editor_cursor_bg: Color,
}

impl ThemeColors {
    pub fn dark() -> Self {
        Self {
            active_border: Color::Rgb(147, 112, 219),
            inactive_border: Color::Rgb(80, 90, 110),
            required: Color::LightRed,
            highlight_bg: Color::Rgb(147, 112, 219),
            accent: Color::Rgb(147, 112, 219),
            text_primary: Color::White,
            text_secondary: Color::DarkGray,
            text_disabled: Color::DarkGray,
            text_on_highlight: Color::Black,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::LightRed,
            error_dark: Color::Red,
            info: Color::Cyan,
            cyan: Color::Cyan,
            bg_main: Color::Rgb(20, 20, 30),
            bg_dialog: Color::Rgb(25, 25, 35),
            bg_list_item: Color::Rgb(30, 30, 40),
            bg_highlight: Color::Rgb(34, 36, 64),
            bg_selected: Color::Rgb(50, 50, 70),
            bg_error: Color::Rgb(50, 20, 20),
            directory: Color::Cyan,
            file: Color::White,
            key: Color::Magenta,
            inactive_text: Color::Rgb(60, 60, 60),
            editor_base_bg: Color::Rgb(20, 20, 30),
            editor_cursor_bg: Color::White,
        }
    }

    pub fn light() -> Self {
        Self {
            active_border: Color::Rgb(99, 102, 241),
            inactive_border: Color::Rgb(148, 163, 184),
            required: Color::Rgb(239, 68, 68),
            highlight_bg: Color::Rgb(99, 102, 241),
            accent: Color::Rgb(139, 92, 246),
            text_primary: Color::Rgb(15, 23, 42),
            text_secondary: Color::Rgb(71, 85, 105),
            text_disabled: Color::Rgb(148, 163, 184),
            text_on_highlight: Color::Rgb(255, 255, 255),
            success: Color::Rgb(16, 185, 129),
            warning: Color::Rgb(245, 158, 11),
            error: Color::Rgb(239, 68, 68),
            error_dark: Color::Rgb(220, 38, 38),
            info: Color::Rgb(37, 99, 235),
            cyan: Color::Rgb(37, 99, 235),
            bg_main: Color::Rgb(248, 250, 252),
            bg_dialog: Color::Rgb(255, 255, 255),
            bg_list_item: Color::Rgb(255, 255, 255),
            bg_highlight: Color::Rgb(224, 231, 255),
            bg_selected: Color::Rgb(199, 210, 254),
            bg_error: Color::Rgb(254, 226, 226),
            directory: Color::Rgb(14, 165, 233),
            file: Color::Rgb(51, 65, 85),
            key: Color::Rgb(168, 85, 247),
            inactive_text: Color::Rgb(148, 163, 184),
            editor_base_bg: Color::Rgb(255, 255, 255),
            editor_cursor_bg: Color::Rgb(30, 41, 59),
        }
    }

    pub fn from_theme(theme: Theme) -> Self {
        match theme {
            Theme::Dark => Self::dark(),
            Theme::Light => Self::light(),
        }
    }
}

pub struct ThemeManager {
    current: RwLock<Theme>,
    colors: RwLock<Arc<ThemeColors>>,
}

impl ThemeManager {
    fn new(theme: Theme) -> Self {
        let colors = Arc::new(ThemeColors::from_theme(theme));
        Self {
            current: RwLock::new(theme),
            colors: RwLock::new(colors),
        }
    }

    pub fn get_colors(&self) -> Arc<ThemeColors> {
        self.colors.read().unwrap().clone()
    }

    pub fn get_theme(&self) -> Theme {
        *self.current.read().unwrap()
    }

    pub fn set_theme(&self, theme: Theme) {
        let colors = Arc::new(ThemeColors::from_theme(theme));
        *self.current.write().unwrap() = theme;
        *self.colors.write().unwrap() = colors;
    }
}

static THEME_MANAGER: OnceLock<ThemeManager> = OnceLock::new();

/// Initialize theme manager (should be called once at startup)
pub fn init_theme_manager(theme: Theme) {
    THEME_MANAGER.get_or_init(|| ThemeManager::new(theme));
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

pub fn set_theme(theme: Theme) {
    get_manager().set_theme(theme);
}
