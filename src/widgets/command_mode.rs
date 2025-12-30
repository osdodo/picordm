use anyhow::Result;
use edtui::{EditorEventHandler, EditorMode, EditorState, EditorTheme, EditorView, Lines};
use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::service::get_redis_service;

#[derive(Debug, Clone)]
pub enum Message {
    UpdateInput(String),
    Execute,
    ToggleFocus,
    EditorKeyEvent(KeyEvent),
}

#[derive(Debug, Clone)]
pub enum Action {
    None,
    ExecuteCommand { command: String },
}

pub struct CommandMode {
    pub command_input: String,
    pub command_output: String,
    pub focus_on_output: bool,
    pub editor_state: EditorState,
    pub editor_handler: EditorEventHandler,
}

impl CommandMode {
    pub fn new() -> Self {
        Self {
            command_input: String::new(),
            command_output: String::new(),
            focus_on_output: false,
            editor_state: EditorState::default(),
            editor_handler: EditorEventHandler::default(),
        }
    }

    pub fn handle_key_events(&self, key: KeyEvent) -> Option<Message> {
        // Tab key handling is the same for both modes
        if key.code == KeyCode::Tab {
            return if !self.command_output.is_empty() {
                Some(Message::ToggleFocus)
            } else {
                None
            };
        }

        if self.focus_on_output {
            match key.code {
                KeyCode::Esc => {
                    // If you press Esc in Normal mode, it returns None, allowing the external process to exit.
                    if self.editor_state.mode == EditorMode::Normal {
                        None
                    } else {
                        // In other modes, the data is passed to the editor for processing.
                        Some(Message::EditorKeyEvent(key))
                    }
                }
                _ => Some(Message::EditorKeyEvent(key)),
            }
        } else {
            match key.code {
                KeyCode::Enter => Some(Message::Execute),
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    Some(Message::UpdateInput(format!("{}{}", self.command_input, c)))
                }
                KeyCode::Backspace => {
                    let mut input = self.command_input.clone();
                    input.pop();
                    Some(Message::UpdateInput(input))
                }
                _ => None,
            }
        }
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::UpdateInput(input) => {
                self.command_input = input;
                Action::None
            }
            Message::Execute => {
                let command = self.command_input.clone();
                if !command.trim().is_empty() {
                    Action::ExecuteCommand { command }
                } else {
                    Action::None
                }
            }
            Message::ToggleFocus => {
                if !self.command_output.is_empty() {
                    self.focus_on_output = !self.focus_on_output;
                }
                Action::None
            }
            Message::EditorKeyEvent(key) => {
                self.editor_handler
                    .on_key_event(key, &mut self.editor_state);
                Action::None
            }
        }
    }

    pub fn view(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Command input
                Constraint::Min(1),    // Output display
            ])
            .split(area);

        self.render_command_input(frame, chunks[0]);
        self.render_command_output(frame, chunks[1]);
    }

    fn render_command_input(&self, frame: &mut Frame, area: Rect) {
        let title = if self.focus_on_output {
            "Command Input (Tab: Switch to Input | Esc: Exit)"
        } else {
            "Command Input (Enter: Execute | Tab: Browse Output | Esc: Exit)"
        };

        let border_color = if self.focus_on_output {
            Color::Rgb(80, 90, 110)
        } else {
            Color::Rgb(147, 112, 219)
        };

        let input_content = format!("> {}", self.command_input);

        let input_widget = Paragraph::new(input_content)
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(border_color))
                    .title(Span::styled(
                        title,
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    )),
            );

        frame.render_widget(input_widget, area);

        if !self.focus_on_output {
            use unicode_width::UnicodeWidthStr;
            let cursor_x = area.x + 3 + self.command_input.width() as u16;
            let cursor_y = area.y + 1;

            if cursor_x < area.right() && cursor_y < area.bottom() {
                frame.set_cursor_position(ratatui::layout::Position {
                    x: cursor_x,
                    y: cursor_y,
                });
            }
        }
    }

    fn render_command_output(&mut self, frame: &mut Frame, area: Rect) {
        let title = if self.command_output.is_empty() {
            "Command Output"
        } else if self.focus_on_output {
            match self.editor_state.mode {
                EditorMode::Visual => {
                    "Command Output (Visual Mode - Esc to exit Visual, then Esc to exit Command Mode)"
                }
                EditorMode::Insert => {
                    "Command Output (Insert Mode - Esc to exit Insert, then Esc to exit Command Mode)"
                }
                EditorMode::Normal => {
                    "Command Output (Browsing - hjkl/arrows to navigate, Tab to return)"
                }
                EditorMode::Search => "Command Output (Search Mode - Esc to exit Search)",
            }
        } else {
            "Command Output (Tab to browse)"
        };

        let border_color = if self.focus_on_output {
            Color::Rgb(147, 112, 219)
        } else {
            Color::Rgb(80, 90, 110)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .title(Span::styled(
                title,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ));

        if self.command_output.is_empty() {
            let paragraph = Paragraph::new("No command executed yet")
                .block(block)
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(paragraph, area);
        } else {
            let theme = EditorTheme {
                block: Some(block),
                ..Default::default()
            };
            let editor_view = EditorView::new(&mut self.editor_state).theme(theme);
            frame.render_widget(editor_view, area);
        }
    }

    pub fn set_command_output(&mut self, output: String, _is_json: bool) {
        self.command_output = output.clone();
        self.editor_state = EditorState::new(Lines::from(output.as_str()));
    }

    pub fn clear_command_input(&mut self) {
        self.command_input.clear();
    }

    pub fn reset(&mut self) {
        self.command_input.clear();
        self.command_output.clear();
        self.focus_on_output = false;
        self.editor_state = EditorState::default();
    }

    pub async fn handle_action(
        &mut self,
        action: Action,
        db_index: u32,
        mut update_footer_error: impl FnMut(Option<String>),
    ) -> Result<()> {
        match action {
            Action::None => Ok(()),
            Action::ExecuteCommand { command } => {
                let parts: Vec<&str> = command.split_whitespace().collect();
                if parts.is_empty() {
                    return Ok(());
                }

                let mut output = match get_redis_service().execute_command(&parts, db_index).await {
                    Ok(output) => {
                        update_footer_error(None);
                        output
                    }
                    Err(e) => {
                        let error_str = e.to_string();
                        if error_str.contains("broken pipe")
                            || error_str.contains("Connection refused")
                            || error_str.contains("Connection reset")
                        {
                            update_footer_error(Some(
                                "Connection lost, please reconnect".to_string(),
                            ));
                        }
                        format!("(error) {}", error_str)
                    }
                };

                // Limit output size
                const MAX_OUTPUT_SIZE: usize = 500 * 1024;
                if output.len() > MAX_OUTPUT_SIZE {
                    let original_len = output.len();
                    output.truncate(MAX_OUTPUT_SIZE);
                    output.push_str(&format!(
                        "\n\n[Output truncated - too large ({} bytes). Use smaller queries or pagination.]",
                        original_len
                    ));
                }

                // Check if it is JSON and format it
                let trimmed = output.trim();
                let (formatted_output, is_json) = if (trimmed.starts_with('{')
                    && trimmed.ends_with('}'))
                    || (trimmed.starts_with('[') && trimmed.ends_with(']'))
                {
                    match serde_json::from_str::<serde_json::Value>(trimmed) {
                        Ok(json_value) => {
                            if let Ok(formatted) = serde_json::to_string_pretty(&json_value) {
                                (formatted, true)
                            } else {
                                (output, false)
                            }
                        }
                        Err(_) => (output, false),
                    }
                } else {
                    (output, false)
                };

                self.set_command_output(formatted_output, is_json);
                self.clear_command_input();
                Ok(())
            }
        }
    }
}
