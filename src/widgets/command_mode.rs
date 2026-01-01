use anyhow::Result;
use edtui::{EditorEventHandler, EditorMode, EditorState, EditorTheme, EditorView, Lines};
use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::service::get_redis_service;
use crate::theme::get_colors;

#[derive(Debug, Clone)]
pub enum Message {
    Enter(u32),
    UpdateInput(String),
    Execute,
    ToggleFocus,
    EditorKeyEvent(KeyEvent),
    Exit,
}

#[derive(Debug, Clone)]
pub enum UpdateResult {
    Continue,
    EnterCommandMode,
    ExitCommandMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Focus {
    Input,
    Output,
}

pub struct CommandMode {
    pub db_index: u32,
    focus: Focus,
    pub command_input: String,
    pub editor_state: EditorState,
    pub editor_handler: EditorEventHandler,
}

impl CommandMode {
    pub fn new() -> Self {
        Self {
            db_index: 0,
            focus: Focus::Input,
            command_input: String::new(),
            editor_state: EditorState::default(),
            editor_handler: EditorEventHandler::default(),
        }
    }

    pub fn handle_key_events(&self, key: KeyEvent) -> Option<Message> {
        if key.code == KeyCode::Tab {
            return if !self.editor_state.lines.is_empty() {
                Some(Message::ToggleFocus)
            } else {
                None
            };
        }

        if key.code == KeyCode::Esc {
            if self.focus == Focus::Output {
                // If in output area and not in Normal mode, pass to editor first
                if self.editor_state.mode != EditorMode::Normal {
                    return Some(Message::EditorKeyEvent(key));
                } else {
                    // Normal mode, Esc exits command mode
                    return Some(Message::Exit);
                }
            } else {
                // Not in output area, Esc exits command mode
                return Some(Message::Exit);
            }
        }

        if self.focus == Focus::Output {
            Some(Message::EditorKeyEvent(key))
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

    pub async fn update(&mut self, msg: Message) -> Result<UpdateResult> {
        match msg {
            Message::Enter(db_index) => {
                self.db_index = db_index;
                self.reset();
                Ok(UpdateResult::EnterCommandMode)
            }
            Message::UpdateInput(input) => {
                self.command_input = input;
                Ok(UpdateResult::Continue)
            }
            Message::Execute => {
                let command = self.command_input.clone();
                if !command.trim().is_empty() {
                    let parts: Vec<&str> = command.split_whitespace().collect();
                    if parts.is_empty() {
                        return Ok(UpdateResult::Continue);
                    }

                    let output = match get_redis_service()
                        .execute_command(&parts, self.db_index)
                        .await
                    {
                        Ok(output) => output,
                        Err(e) => format!("(error) {}", e),
                    };

                    self.editor_state = EditorState::new(Lines::from(output.as_str()));
                    self.command_input.clear();
                }
                Ok(UpdateResult::Continue)
            }
            Message::ToggleFocus => {
                if !self.editor_state.lines.is_empty() {
                    self.focus = match self.focus {
                        Focus::Input => Focus::Output,
                        Focus::Output => Focus::Input,
                    };
                }
                Ok(UpdateResult::Continue)
            }
            Message::EditorKeyEvent(key) => {
                self.editor_handler
                    .on_key_event(key, &mut self.editor_state);
                Ok(UpdateResult::Continue)
            }
            Message::Exit => Ok(UpdateResult::ExitCommandMode),
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
        let colors = get_colors();

        let title = match self.focus {
            Focus::Output => "Command Input (Tab: Switch to Input | Esc: Exit)",
            Focus::Input => "Command Input (Enter: Execute | Tab: Browse Output | Esc: Exit)",
        };

        let border_color = match self.focus {
            Focus::Output => colors.border_default,
            Focus::Input => colors.border_active,
        };

        let input_content = format!("> {}", self.command_input);

        let input_widget = Paragraph::new(input_content)
            .style(Style::default().fg(colors.text_primary))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(border_color))
                    .title(Span::styled(
                        title,
                        Style::default()
                            .fg(colors.text_primary)
                            .add_modifier(Modifier::BOLD),
                    )),
            );

        frame.render_widget(input_widget, area);

        if self.focus == Focus::Input {
            use unicode_width::UnicodeWidthStr;
            let cursor_x = area.x + 3 + self.command_input.width() as u16;
            let cursor_y = area.y + 1;

            if cursor_x < area.right() && cursor_y < area.bottom() {
                frame.set_cursor_position(Position {
                    x: cursor_x,
                    y: cursor_y,
                });
            }
        }
    }

    fn render_command_output(&mut self, frame: &mut Frame, area: Rect) {
        let colors = get_colors();

        let title = if self.editor_state.lines.is_empty() {
            "Command Output"
        } else {
            match (self.focus, self.editor_state.mode) {
                (Focus::Output, EditorMode::Visual) => {
                    "Command Output (Visual Mode - Esc to exit Visual, then Esc to exit Command Mode)"
                }
                (Focus::Output, EditorMode::Insert) => {
                    "Command Output (Insert Mode - Esc to exit Insert, then Esc to exit Command Mode)"
                }
                (Focus::Output, EditorMode::Normal) => {
                    "Command Output (Browsing - hjkl/arrows to navigate, Tab to return)"
                }
                (Focus::Output, EditorMode::Search) => {
                    "Command Output (Search Mode - Esc to exit Search)"
                }
                (Focus::Input, _) => "Command Output (Tab to browse)",
            }
        };

        let border_color = match self.focus {
            Focus::Output => colors.border_active,
            Focus::Input => colors.border_default,
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .title(Span::styled(
                title,
                Style::default()
                    .fg(colors.text_primary)
                    .add_modifier(Modifier::BOLD),
            ));

        if self.editor_state.lines.is_empty() {
            let paragraph = Paragraph::new("No command executed yet")
                .block(block)
                .style(Style::default().fg(colors.text_secondary));
            frame.render_widget(paragraph, area);
        } else {
            let base_style = Style::default()
                .bg(colors.editor_base_bg)
                .fg(colors.text_primary);
            let cursor_style = Style::default().bg(colors.editor_cursor_bg);

            let theme = EditorTheme {
                block: Some(block),
                base: base_style,
                cursor_style,
                status_line: None,
                ..Default::default()
            };
            let editor_view = EditorView::new(&mut self.editor_state).theme(theme);
            frame.render_widget(editor_view, area);
        }
    }

    fn reset(&mut self) {
        self.command_input.clear();
        self.editor_state = EditorState::default();
        self.focus = Focus::Input;
    }
}
