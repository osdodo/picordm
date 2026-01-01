use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::theme::get_colors;

#[derive(Debug, Clone)]
pub enum Message {
    Show { title: String, message: String },
    Confirm,
    Cancel,
}

pub struct ConfirmDialog {
    pub title: String,
    pub message: String,
    pub confirm_label: String,
    pub cancel_label: String,
    pub is_open: bool,
}

impl ConfirmDialog {
    pub fn new() -> Self {
        Self {
            title: "Confirm".to_string(),
            message: String::new(),
            confirm_label: "Yes".to_string(),
            cancel_label: "Cancel".to_string(),
            is_open: false,
        }
    }

    pub fn handle_key_events(&self, key: KeyEvent) -> Option<Message> {
        if !self.is_open {
            return None;
        }
        match key.code {
            KeyCode::Char('y') | KeyCode::Enter => Some(Message::Confirm),
            KeyCode::Char('n') | KeyCode::Esc => Some(Message::Cancel),
            _ => None,
        }
    }

    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::Show { title, message } => {
                self.title = title;
                self.message = message;
                self.is_open = true;
            }
            Message::Confirm => {
                self.is_open = false;
            }
            Message::Cancel => {
                self.is_open = false;
            }
        }
    }

    pub fn view(&self, frame: &mut Frame, area: Rect) {
        if !self.is_open {
            return;
        }

        let colors = get_colors();

        let block = Block::default()
            .title(Line::from(vec![Span::styled(
                &self.title,
                Style::default()
                    .fg(colors.text_primary)
                    .add_modifier(Modifier::BOLD),
            )]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(colors.border_active))
            .style(Style::default().bg(colors.bg_dialog));

        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Min(1),    // Top spacing
                Constraint::Min(1),    // Message (can be multiple lines)
                Constraint::Min(1),    // Middle spacing
                Constraint::Length(1), // Buttons hint
            ])
            .split(area);

        let message_widget = Paragraph::new(Line::from(Span::styled(
            &self.message,
            Style::default()
                .fg(colors.text_primary)
                .add_modifier(Modifier::BOLD),
        )))
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(ratatui::widgets::Wrap { trim: true });

        frame.render_widget(message_widget, chunks[1]);

        let buttons_hint = Paragraph::new(Line::from(vec![
            Span::styled(
                "[y]",
                Style::default()
                    .fg(colors.error)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {}, ", self.confirm_label),
                Style::default().fg(colors.text_primary),
            ),
            Span::styled(
                "[n/Esc]",
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {}", self.cancel_label),
                Style::default().fg(colors.text_primary),
            ),
        ]))
        .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(buttons_hint, chunks[3]);
    }
}
