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
    Confirm,
    Cancel,
}

pub struct DeleteDialog {
    pub selected_count: usize,
    pub is_open: bool,
}

impl DeleteDialog {
    pub fn new() -> Self {
        Self {
            selected_count: 0,
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
                "Confirm Delete",
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
                Constraint::Length(1), // Message
                Constraint::Min(1),    // Middle spacing
                Constraint::Length(1), // Buttons hint
            ])
            .split(area);

        let message = format!(
            "Are you sure you want to delete {} key{}?",
            self.selected_count,
            if self.selected_count == 1 { "" } else { "s" }
        );

        let message_widget = Paragraph::new(Line::from(Span::styled(
            message,
            Style::default()
                .fg(colors.text_primary)
                .add_modifier(Modifier::BOLD),
        )))
        .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(message_widget, chunks[1]);

        let buttons_hint = Paragraph::new(Line::from(vec![
            Span::styled(
                "[y]",
                Style::default()
                    .fg(colors.error)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Yes, delete  ", Style::default().fg(colors.text_primary)),
            Span::styled(
                "[n/Esc]",
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Cancel", Style::default().fg(colors.text_primary)),
        ]))
        .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(buttons_hint, chunks[3]);
    }

    pub fn open(&mut self, count: usize) {
        self.selected_count = count;
        self.is_open = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }
}
