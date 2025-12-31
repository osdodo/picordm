use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph},
};

use crate::screens::utils::centered_rect_fixed_height;
use crate::theme::{Theme, get_colors, get_theme, set_theme};
use crate::widgets::settings_storage::save_theme;

pub struct SettingsDialog {
    pub is_open: bool,
    pub selected_theme: Theme,
    pub state: ListState,
}

impl SettingsDialog {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));

        Self {
            is_open: false,
            selected_theme: Theme::Dark,
            state,
        }
    }

    pub fn show(&mut self) {
        self.is_open = true;
        self.selected_theme = get_theme();
        self.state.select(Some(match self.selected_theme {
            Theme::Dark => 0,
            Theme::Light => 1,
        }));
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> bool {
        if !self.is_open {
            return false;
        }

        match key.code {
            KeyCode::Esc => {
                self.close();
                true
            }
            KeyCode::Enter => {
                set_theme(self.selected_theme);
                if let Err(e) = save_theme(self.selected_theme) {
                    eprintln!("Failed to save theme: {}", e);
                }
                self.close();
                true
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let current = self.state.selected().unwrap_or(0);
                let new = if current == 0 { 1 } else { 0 };
                self.selected_theme = if new == 0 { Theme::Dark } else { Theme::Light };
                self.state.select(Some(new));
                set_theme(self.selected_theme);
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let current = self.state.selected().unwrap_or(0);
                let new = if current == 0 { 1 } else { 0 };
                self.selected_theme = if new == 0 { Theme::Dark } else { Theme::Light };
                self.state.select(Some(new));
                set_theme(self.selected_theme);
                true
            }
            KeyCode::Char('1') => {
                self.selected_theme = Theme::Dark;
                self.state.select(Some(0));
                set_theme(self.selected_theme);
                true
            }
            KeyCode::Char('2') => {
                self.selected_theme = Theme::Light;
                self.state.select(Some(1));
                set_theme(self.selected_theme);
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

        let popup_area = centered_rect_fixed_height(50, 12, frame.area());
        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .title("Settings")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(colors.active_border))
            .style(Style::default().bg(colors.bg_dialog));

        frame.render_widget(block, popup_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(4),
                Constraint::Length(3),
            ])
            .split(popup_area);

        let title = Paragraph::new(Line::from(vec![Span::styled(
            "Select Theme",
            Style::default()
                .fg(colors.text_primary)
                .add_modifier(Modifier::BOLD),
        )]))
        .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(title, chunks[0]);

        let themes = vec![
            ListItem::new(Line::from(vec![
                Span::styled(
                    if self.selected_theme == Theme::Dark {
                        "● "
                    } else {
                        "○ "
                    },
                    Style::default().fg(colors.active_border),
                ),
                Span::styled("Dark Theme", Style::default().fg(colors.text_primary)),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled(
                    if self.selected_theme == Theme::Light {
                        "● "
                    } else {
                        "○ "
                    },
                    Style::default().fg(colors.active_border),
                ),
                Span::styled("Light Theme", Style::default().fg(colors.text_primary)),
            ])),
        ];

        let list = List::new(themes)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(colors.inactive_border)),
            )
            .highlight_style(
                Style::default()
                    .bg(colors.highlight_bg)
                    .fg(colors.text_on_highlight)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, chunks[1], &mut self.state.clone());

        let instructions = Paragraph::new(vec![Line::from(vec![
            Span::styled("↑↓", Style::default().fg(colors.info)),
            Span::styled(" Navigate  ", Style::default().fg(colors.text_primary)),
            Span::styled("Enter", Style::default().fg(colors.info)),
            Span::styled(" Confirm  ", Style::default().fg(colors.text_primary)),
            Span::styled("Esc", Style::default().fg(colors.error_dark)),
            Span::styled(" Cancel", Style::default().fg(colors.text_primary)),
        ])])
        .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(instructions, chunks[2]);
    }
}
