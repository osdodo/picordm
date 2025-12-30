use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

#[derive(Debug, Clone)]
pub enum Message {
    Show(String, String),
    Complete(String),
    Hide,
}

pub struct ProgressDialog {
    pub title: String,
    pub message: String,
    pub is_complete: bool,
    pub progress: Option<(usize, usize)>,
    pub completed_at: Option<std::time::Instant>,
    pub is_visible: bool,
}

impl ProgressDialog {
    pub fn new() -> Self {
        Self {
            title: String::new(),
            message: String::new(),
            is_complete: false,
            progress: None,
            completed_at: None,
            is_visible: false,
        }
    }

    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::Show(title, message) => {
                self.title = title;
                self.message = message;
                self.is_complete = false;
                self.progress = None;
                self.completed_at = None;
                self.is_visible = true;
            }
            Message::Complete(message) => {
                self.message = message;
                self.is_complete = true;
                self.completed_at = Some(std::time::Instant::now());
            }
            Message::Hide => {
                self.is_visible = false;
            }
        }
    }

    pub fn view(&self, frame: &mut Frame, area: Rect) {
        if !self.is_visible {
            return;
        }

        let title_color = if self.is_complete {
            Color::Green
        } else {
            Color::Rgb(147, 112, 219)
        };

        let block = Block::default()
            .title(Line::from(vec![Span::styled(
                &self.title,
                Style::default()
                    .fg(title_color)
                    .add_modifier(Modifier::BOLD),
            )]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(title_color))
            .style(Style::default().bg(Color::Rgb(25, 25, 35)));

        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(2), // Message with more space
                Constraint::Length(1), // Progress indicator
            ])
            .split(area);

        // Message with icon and better styling
        let message_lines = if self.is_complete {
            let message_lower = self.message.to_lowercase();
            vec![
                Line::from(""),
                Line::from(vec![Span::styled(
                    &self.message,
                    Style::default()
                        .fg(
                            if message_lower.contains("exported")
                                || message_lower.contains("imported")
                            {
                                Color::Green
                            } else {
                                Color::Red
                            },
                        )
                        .add_modifier(Modifier::BOLD),
                )]),
            ]
        } else {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    &self.message,
                    Style::default().fg(Color::White),
                )),
            ]
        };

        let message_widget =
            Paragraph::new(message_lines).alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(message_widget, chunks[0]);

        // Bottom hint - only show when complete
        if self.is_complete {
            let hint_widget = Paragraph::new(Line::from(vec![
                Span::styled("Press ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "Esc",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" to close", Style::default().fg(Color::DarkGray)),
            ]))
            .alignment(ratatui::layout::Alignment::Center);
            frame.render_widget(hint_widget, chunks[1]);
        }
    }
}
