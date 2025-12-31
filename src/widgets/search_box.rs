use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use unicode_width::UnicodeWidthStr;

use crate::theme::get_colors;

#[derive(Debug, Clone)]
pub enum Message {
    UpdateText(String),
    ToggleFocus,
}

pub struct SearchBox {
    pub text: String,
    pub is_focused: bool,
    pub placeholder: String,
}

impl SearchBox {
    pub fn new(placeholder: &str) -> Self {
        Self {
            text: String::new(),
            is_focused: false,
            placeholder: placeholder.to_string(),
        }
    }

    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::UpdateText(text) => {
                self.text = text;
            }
            Message::ToggleFocus => {
                self.is_focused = !self.is_focused;
            }
        }
    }

    pub fn handle_key_events(&self, key: KeyEvent) -> Option<Message> {
        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => Some(Message::ToggleFocus),
            (KeyCode::Enter, _) => Some(Message::ToggleFocus),
            (KeyCode::Char('>'), _) => {
                let new_text = format!("{}{}", self.text, '>');
                Some(Message::UpdateText(new_text))
            }
            (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                let new_text = format!("{}{}", self.text, c);
                Some(Message::UpdateText(new_text))
            }
            (KeyCode::Backspace, _) => {
                let mut text = self.text.clone();
                text.pop();
                Some(Message::UpdateText(text))
            }
            _ => None,
        }
    }

    pub fn view(&self, frame: &mut Frame, area: Rect) -> Option<(u16, u16)> {
        let border_color = if self.is_focused {
            get_colors().active_border
        } else {
            get_colors().inactive_border
        };

        let title = if self.is_focused {
            format!("{} (Esc to exit)", self.placeholder)
        } else {
            format!("{} (Press '/' to search)", self.placeholder)
        };

        let display = if self.text.is_empty() && !self.is_focused {
            Span::styled(
                "...",
                Style::default()
                    .fg(get_colors().text_secondary)
                    .add_modifier(Modifier::DIM),
            )
        } else {
            Span::styled(&self.text, Style::default().fg(get_colors().text_primary))
        };

        let search_input = Paragraph::new(Line::from(vec![display])).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .title(Span::styled(
                    title,
                    Style::default()
                        .fg(get_colors().text_primary)
                        .add_modifier(Modifier::BOLD),
                )),
        );

        frame.render_widget(search_input, area);

        // Return cursor position if actively searching
        if self.is_focused {
            Some((area.x + 1 + self.text.width() as u16, area.y + 1))
        } else {
            None
        }
    }
}
