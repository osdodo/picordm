use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::theme::get_colors;

#[derive(Debug, Clone)]
pub enum Message {
    Show(String, String),
    Complete(String, Option<Vec<String>>),
    Hide,
}

pub struct ProgressDialog {
    pub title: String,
    pub message: String,
    pub error_details: Vec<String>,
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
            error_details: Vec::new(),
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
                self.error_details.clear();
                self.is_complete = false;
                self.progress = None;
                self.completed_at = None;
                self.is_visible = true;
            }
            Message::Complete(message, errors) => {
                self.message = message;
                self.error_details = errors.unwrap_or_default();
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

        let error_height = if !self.error_details.is_empty() {
            // Calculate height needed for error details (max 5 lines)
            (self.error_details.len().min(5) + 1) as u16
        } else {
            0
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(2), // Message
                if error_height > 0 {
                    Constraint::Length(error_height)
                } else {
                    Constraint::Length(1)
                }, // Error details or hint
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
                                colors.success
                            } else {
                                colors.error
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
                    Style::default().fg(colors.text_primary),
                )),
            ]
        };

        let message_widget =
            Paragraph::new(message_lines).alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(message_widget, chunks[0]);

        // Show error details or hint
        if !self.error_details.is_empty() {
            let mut error_lines = vec![Line::from(vec![Span::styled(
                "Failed keys: ",
                Style::default()
                    .fg(colors.error)
                    .add_modifier(Modifier::BOLD),
            )])];

            // Show up to 5 failed keys
            for error in self.error_details.iter().take(5) {
                error_lines.push(Line::from(vec![
                    Span::styled("  • ", Style::default().fg(colors.error)),
                    Span::styled(error, Style::default().fg(colors.error)),
                ]));
            }

            if self.error_details.len() > 5 {
                error_lines.push(Line::from(vec![Span::styled(
                    format!("  ... and {} more", self.error_details.len() - 5),
                    Style::default().fg(colors.text_secondary),
                )]));
            }

            let error_widget = Paragraph::new(error_lines)
                .alignment(ratatui::layout::Alignment::Left)
                .wrap(ratatui::widgets::Wrap { trim: true });
            frame.render_widget(error_widget, chunks[1]);
        } else if self.is_complete {
            let hint_widget = Paragraph::new(Line::from(vec![
                Span::styled("Press ", Style::default().fg(colors.text_secondary)),
                Span::styled(
                    "Esc",
                    Style::default()
                        .fg(colors.text_primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" to close", Style::default().fg(colors.text_secondary)),
            ]))
            .alignment(ratatui::layout::Alignment::Center);
            frame.render_widget(hint_widget, chunks[1]);
        }
    }
}
