use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use crate::screens::utils::centered_rect_fixed_size;
use crate::theme::{
    Theme, get_colors, get_enable_bg_transparent, get_theme, set_enable_bg_transparent, set_theme,
};
use crate::widgets::settings_storage::{AppSettings, save_settings};

pub struct SettingsDialog {
    pub is_open: bool,
    pub selected_theme: Theme,
    pub enable_bg_transparent: bool,
    pub focus: SettingsFocus,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsFocus {
    Theme,
    Transparency,
}

impl SettingsDialog {
    pub fn new() -> Self {
        Self {
            is_open: false,
            selected_theme: Theme::Dark,
            enable_bg_transparent: false,
            focus: SettingsFocus::Theme,
        }
    }

    pub fn show(&mut self) {
        self.is_open = true;
        self.selected_theme = get_theme();
        self.enable_bg_transparent = get_enable_bg_transparent();
        self.focus = SettingsFocus::Theme;
    }

    pub fn close(&mut self) {
        // Auto-save settings when closing
        let settings = AppSettings {
            theme: self.selected_theme,
            enable_bg_transparent: self.enable_bg_transparent,
        };
        if let Err(e) = save_settings(&settings) {
            eprintln!("Failed to save settings: {}", e);
        }
        self.is_open = false;
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> bool {
        if !self.is_open {
            return false;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.close();
                true
            }
            KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => {
                self.focus = match self.focus {
                    SettingsFocus::Theme => SettingsFocus::Transparency,
                    SettingsFocus::Transparency => SettingsFocus::Theme,
                };
                true
            }
            KeyCode::BackTab | KeyCode::Up | KeyCode::Char('k') => {
                self.focus = match self.focus {
                    SettingsFocus::Theme => SettingsFocus::Transparency,
                    SettingsFocus::Transparency => SettingsFocus::Theme,
                };
                true
            }
            KeyCode::Left | KeyCode::Char('h') => {
                match self.focus {
                    SettingsFocus::Theme => {
                        self.selected_theme = Theme::Dark;
                        set_theme(self.selected_theme);
                        // Auto-enable transparency when switching to Dark theme
                        self.enable_bg_transparent = true;
                        set_enable_bg_transparent(true);
                    }
                    SettingsFocus::Transparency => {
                        if self.selected_theme == Theme::Dark {
                            self.enable_bg_transparent = false;
                            set_enable_bg_transparent(false);
                        }
                    }
                }
                true
            }
            KeyCode::Right | KeyCode::Char('l') => {
                match self.focus {
                    SettingsFocus::Theme => {
                        self.selected_theme = Theme::Light;
                        set_theme(self.selected_theme);
                        // Auto-disable transparency when switching to Light theme
                        if self.enable_bg_transparent {
                            self.enable_bg_transparent = false;
                            set_enable_bg_transparent(false);
                        }
                    }
                    SettingsFocus::Transparency => {
                        if self.selected_theme == Theme::Dark {
                            self.enable_bg_transparent = true;
                            set_enable_bg_transparent(true);
                        }
                    }
                }
                true
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                match self.focus {
                    SettingsFocus::Theme => {
                        // Toggle theme
                        self.selected_theme = match self.selected_theme {
                            Theme::Dark => Theme::Light,
                            Theme::Light => Theme::Dark,
                        };
                        set_theme(self.selected_theme);
                        // Auto-enable transparency when switching to Dark theme
                        if self.selected_theme == Theme::Dark {
                            self.enable_bg_transparent = true;
                            set_enable_bg_transparent(true);
                        } else if self.selected_theme == Theme::Light && self.enable_bg_transparent
                        {
                            // Auto-disable transparency when switching to Light theme
                            self.enable_bg_transparent = false;
                            set_enable_bg_transparent(false);
                        }
                    }
                    SettingsFocus::Transparency => {
                        if self.selected_theme == Theme::Dark {
                            self.enable_bg_transparent = !self.enable_bg_transparent;
                            set_enable_bg_transparent(self.enable_bg_transparent);
                        }
                    }
                }
                true
            }
            _ => false,
        }
    }

    pub fn view(&self, frame: &mut Frame) {
        if !self.is_open {
            return;
        }

        let colors = get_colors();
        let popup_area = centered_rect_fixed_size(110, 18, frame.area());

        frame.render_widget(Clear, popup_area);

        let main_block = Block::default()
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    "Settings",
                    Style::default()
                        .fg(colors.text_primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
            ]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(colors.border_default))
            .style(Style::default().bg(colors.bg_dialog));

        frame.render_widget(main_block, popup_area);

        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(4), // Theme section
                Constraint::Length(1), // Spacer
                Constraint::Length(4), // Transparency section
                Constraint::Min(0),    // Flexible space
                Constraint::Length(1), // Help text
            ])
            .split(popup_area);

        // === THEME SECTION ===
        self.render_theme_section(frame, main_chunks[0], &colors);

        // === TRANSPARENCY SECTION ===
        self.render_transparency_section(frame, main_chunks[2], &colors);

        // === HELP TEXT ===
        self.render_help_text(frame, main_chunks[4], &colors);
    }

    fn render_theme_section(
        &self,
        frame: &mut Frame,
        area: Rect,
        colors: &crate::theme::ThemeColors,
    ) {
        let is_focused = self.focus == SettingsFocus::Theme;

        let border_color = if is_focused {
            colors.border_active
        } else {
            colors.border_default
        };

        let block = Block::default()
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    "Theme",
                    Style::default()
                        .fg(colors.text_primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
            ]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));

        frame.render_widget(block, area);

        let inner = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area);

        let desc = Paragraph::new(Line::from(vec![Span::styled(
            "Choose your preferred color scheme",
            Style::default().fg(colors.text_secondary),
        )]));
        frame.render_widget(desc, inner[0]);

        let dark_selected = self.selected_theme == Theme::Dark;
        let light_selected = self.selected_theme == Theme::Light;

        let options = Paragraph::new(Line::from(vec![
            Span::styled(
                if dark_selected { "[●] " } else { "[ ] " },
                Style::default().fg(if dark_selected {
                    colors.accent
                } else {
                    colors.text_secondary
                }),
            ),
            Span::styled(
                "Dark",
                Style::default().fg(if dark_selected {
                    colors.text_primary
                } else {
                    colors.text_secondary
                }),
            ),
            Span::raw("    "),
            Span::styled(
                if light_selected { "[●] " } else { "[ ] " },
                Style::default().fg(if light_selected {
                    colors.accent
                } else {
                    colors.text_secondary
                }),
            ),
            Span::styled(
                "Light",
                Style::default().fg(if light_selected {
                    colors.text_primary
                } else {
                    colors.text_secondary
                }),
            ),
        ]))
        .alignment(Alignment::Left);

        frame.render_widget(options, inner[1]);
    }

    fn render_transparency_section(
        &self,
        frame: &mut Frame,
        area: Rect,
        colors: &crate::theme::ThemeColors,
    ) {
        let is_focused = self.focus == SettingsFocus::Transparency;
        let is_disabled = self.selected_theme == Theme::Light;

        let border_color = if is_focused && !is_disabled {
            colors.border_active
        } else {
            colors.border_default
        };

        let block = Block::default()
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    "Background",
                    Style::default()
                        .fg(if is_disabled {
                            colors.text_disabled
                        } else {
                            colors.text_primary
                        })
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
            ]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));

        frame.render_widget(block, area);

        let inner = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area);

        // Checkbox line
        let checkbox = if self.enable_bg_transparent && !is_disabled {
            "[√]"
        } else {
            "[ ]"
        };
        let checkbox_line = Line::from(vec![
            Span::styled(
                checkbox,
                Style::default().fg(if is_disabled {
                    colors.text_disabled
                } else {
                    colors.accent
                }),
            ),
            Span::raw(" "),
            Span::styled(
                "Enable transparent background",
                Style::default().fg(if is_disabled {
                    colors.text_disabled
                } else {
                    colors.text_primary
                }),
            ),
        ]);
        frame.render_widget(Paragraph::new(checkbox_line), inner[0]);

        let info_text = if is_disabled {
            "Only available in Dark theme"
        } else {
            "Requires terminal transparency support"
        };
        let info_line = Line::from(vec![Span::styled(
            info_text,
            Style::default()
                .fg(colors.text_secondary)
                .add_modifier(Modifier::ITALIC),
        )]);
        frame.render_widget(Paragraph::new(info_line), inner[1]);
    }

    fn render_help_text(&self, frame: &mut Frame, area: Rect, colors: &crate::theme::ThemeColors) {
        let help = Line::from(vec![
            Span::styled("Tab/↑↓", Style::default().fg(colors.accent)),
            Span::styled(" Navigate  ", Style::default().fg(colors.text_secondary)),
            Span::styled("←→/Space", Style::default().fg(colors.accent)),
            Span::styled(" Toggle  ", Style::default().fg(colors.text_secondary)),
            Span::styled("Esc", Style::default().fg(colors.error)),
            Span::styled(" Close/Save", Style::default().fg(colors.text_secondary)),
        ]);

        let help_widget = Paragraph::new(help)
            .alignment(Alignment::Center)
            .style(Style::default().fg(colors.text_secondary));

        frame.render_widget(help_widget, area);
    }
}
