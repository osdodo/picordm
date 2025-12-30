use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

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

        let block = Block::default()
            .title(Line::from(vec![Span::styled(
                "⚠ Confirm Delete",
                Style::default()
                    .fg(Color::LightRed)
                    .add_modifier(Modifier::BOLD),
            )]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::LightRed))
            .style(Style::default().bg(Color::Rgb(30, 30, 40)));

        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(2), // Message
                Constraint::Length(1), // Buttons hint
            ])
            .split(area);

        let message = if self.selected_count == 1 {
            "Are you sure you want to delete 1 key?".to_string()
        } else {
            format!(
                "Are you sure you want to delete {} keys?",
                self.selected_count
            )
        };

        let message_widget = Paragraph::new(vec![
            Line::from(Span::styled(
                message,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "This action cannot be undone.",
                Style::default().fg(Color::Yellow),
            )),
        ])
        .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(message_widget, chunks[0]);

        let buttons_hint = Paragraph::new(Line::from(vec![
            Span::styled(
                "[y]",
                Style::default()
                    .fg(Color::LightRed)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Yes, delete  ", Style::default().fg(Color::White)),
            Span::styled(
                "[n/Esc]",
                Style::default()
                    .fg(Color::Rgb(147, 112, 219))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Cancel", Style::default().fg(Color::White)),
        ]))
        .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(buttons_hint, chunks[1]);
    }

    pub fn open(&mut self, count: usize) {
        self.selected_count = count;
        self.is_open = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }
}
