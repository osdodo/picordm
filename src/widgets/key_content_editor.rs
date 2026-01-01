use edtui::{
    EditorEventHandler, EditorMode, EditorState, EditorTheme, EditorView, Lines, SyntaxHighlighter,
};
use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

use crate::theme::{Theme, get_colors, get_theme};

#[derive(Debug, Clone)]
pub enum Message {
    EditorKeyEvent(KeyEvent),
    VimCommandModeToggle,
    VimCommandModeUpdateInput(String),
    VimCommandModeExecute,
    Exit,
    SetLoadingValue(bool),
    LoadKeyValue(String, String),
}

#[derive(Debug, Clone)]
pub enum UpdateResult {
    None,
    Save,
    SaveAndQuit,
    Quit,
}

pub struct KeyContentEditor {
    pub editor_state: EditorState,
    pub editor_handler: EditorEventHandler,
    pub current_key: Option<String>,
    pub is_json: bool,
    pub is_vim_command_mode: bool,
    pub vim_command_input: String,
    pub is_loading_value: bool,
}

impl KeyContentEditor {
    pub fn new() -> Self {
        Self {
            editor_state: EditorState::default(),
            editor_handler: EditorEventHandler::default(),
            current_key: None,
            is_json: false,
            is_vim_command_mode: false,
            vim_command_input: String::new(),
            is_loading_value: false,
        }
    }

    pub fn handle_key_events(&self, key: KeyEvent) -> Option<Message> {
        if self.is_vim_command_mode {
            return match key.code {
                KeyCode::Enter => Some(Message::VimCommandModeExecute),
                KeyCode::Esc => Some(Message::VimCommandModeToggle),
                KeyCode::Char(c) => Some(Message::VimCommandModeUpdateInput(format!(
                    "{}{}",
                    self.vim_command_input, c
                ))),
                KeyCode::Backspace => {
                    let mut text = self.vim_command_input.clone();
                    text.pop();
                    Some(Message::VimCommandModeUpdateInput(text))
                }
                _ => None,
            };
        }

        // Handle ':' key for vim command mode - only in Normal mode
        // On Windows, ':' requires Shift, so we need to allow SHIFT modifier
        if self.editor_state.mode == EditorMode::Normal
            && let KeyCode::Char(':') = key.code
        {
            // Allow ':' with or without Shift modifier (Windows compatibility)
            let has_other_modifiers = key.modifiers.contains(KeyModifiers::CONTROL)
                || key.modifiers.contains(KeyModifiers::ALT);
            if !has_other_modifiers {
                return Some(Message::VimCommandModeToggle);
            }
        }

        if key.code == KeyCode::Esc {
            match self.editor_state.mode {
                EditorMode::Insert | EditorMode::Visual | EditorMode::Search => {
                    // Esc returns to Normal mode.
                    return Some(Message::EditorKeyEvent(key));
                }
                EditorMode::Normal => {
                    // Normal mode, Esc exits editor.
                    return Some(Message::Exit);
                }
            }
        }

        Some(Message::EditorKeyEvent(key))
    }

    pub fn update(&mut self, msg: Message) -> UpdateResult {
        match msg {
            Message::SetLoadingValue(loading) => {
                self.is_loading_value = loading;
                UpdateResult::None
            }
            Message::LoadKeyValue(key, value) => {
                self.current_key = Some(key);
                let looks_like_json = value.trim_start().starts_with(['{', '[', '"']);
                let (content, is_json) = if looks_like_json {
                    serde_json::from_str::<serde_json::Value>(&value)
                        .ok()
                        .and_then(|json| serde_json::to_string_pretty(&json).ok())
                        .map(|pretty| (pretty, true))
                        .unwrap_or_else(|| (value, false))
                } else {
                    (value, false)
                };
                self.is_json = is_json;
                self.editor_state = EditorState::new(Lines::from(content.as_str()));
                self.is_loading_value = false;
                UpdateResult::None
            }
            Message::EditorKeyEvent(key) => {
                self.editor_handler
                    .on_key_event(key, &mut self.editor_state);
                UpdateResult::None
            }
            Message::VimCommandModeToggle => {
                self.is_vim_command_mode = !self.is_vim_command_mode;
                if self.is_vim_command_mode {
                    self.vim_command_input.clear();
                }
                UpdateResult::None
            }
            Message::VimCommandModeUpdateInput(input) => {
                self.vim_command_input = input;
                UpdateResult::None
            }
            Message::VimCommandModeExecute => {
                let cmd = self.vim_command_input.trim();
                let result = match cmd {
                    "w" => UpdateResult::Save,
                    "q" => UpdateResult::Quit,
                    "wq" => UpdateResult::SaveAndQuit,
                    "q!" => UpdateResult::Quit,
                    _ => UpdateResult::None,
                };

                self.is_vim_command_mode = false;
                self.vim_command_input.clear();

                result
            }
            Message::Exit => UpdateResult::Quit,
        }
    }

    pub fn view(&mut self, frame: &mut Frame, area: Rect) {
        let colors = get_colors();

        let title = if self.is_vim_command_mode {
            Line::from(vec![Span::styled(
                "Vim Command Mode (Enter to execute, Esc to cancel)",
                Style::default()
                    .fg(colors.text_primary)
                    .add_modifier(Modifier::BOLD),
            )])
        } else if !self.editor_state.lines.is_empty() {
            let (mode_name, mode_color) = match self.editor_state.mode {
                EditorMode::Normal => ("NORMAL", colors.success),
                EditorMode::Insert => ("INSERT", colors.info),
                EditorMode::Visual => ("VISUAL", colors.warning),
                EditorMode::Search => ("SEARCH", colors.info),
            };
            let mode_hint = match self.editor_state.mode {
                EditorMode::Normal => " (Press ':' for commands (:w=Save :q=Quit :wq=Save&Quit))",
                EditorMode::Insert => " (Press Esc to Normal mode, then ':' for commands)",
                EditorMode::Visual => " (Press Esc to Normal mode, then ':' for commands)",
                EditorMode::Search => " (Press Esc to Normal mode, then ':' for commands)",
            };
            Line::from(vec![
                Span::styled(
                    format!("[{}]", mode_name),
                    Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(mode_hint, Style::default().fg(colors.text_secondary)),
            ])
        } else {
            Line::from(vec![Span::styled(
                "View",
                Style::default()
                    .fg(colors.text_primary)
                    .add_modifier(Modifier::BOLD),
            )])
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(colors.border_active))
            .title(title);

        if self.is_loading_value {
            let loading_text = Span::styled("Loading value...", Style::default().fg(colors.info));
            let paragraph = Paragraph::new(loading_text)
                .block(block)
                .alignment(ratatui::layout::Alignment::Center);
            frame.render_widget(paragraph, area);
        } else if !self.editor_state.lines.is_empty() {
            let (render_area, cmd_area) = if self.is_vim_command_mode {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1), Constraint::Length(1)])
                    .split(area.inner(Margin::new(1, 1)));
                (chunks[0], Some(chunks[1]))
            } else {
                (area, None)
            };

            let base_style = Style::default()
                .bg(colors.editor_base_bg)
                .fg(colors.text_primary);
            let cursor_style = Style::default().bg(colors.editor_cursor_bg);

            let editor_theme = EditorTheme {
                block: if self.is_vim_command_mode {
                    None
                } else {
                    Some(block.clone())
                },
                base: base_style,
                cursor_style,
                status_line: None,
                ..Default::default()
            };

            let mut editor_view = EditorView::new(&mut self.editor_state).theme(editor_theme);

            if self.is_json {
                let syntax_theme = match get_theme() {
                    Theme::Dark => "visual-studio-dark",
                    Theme::Light => "inspired-github",
                };
                let syntax_highlighter = SyntaxHighlighter::new(syntax_theme, "json");
                editor_view = editor_view.syntax_highlighter(Some(syntax_highlighter));
            }

            frame.render_widget(editor_view, render_area);

            if let Some(cmd_area) = cmd_area {
                let cmd_line = format!(":{}", self.vim_command_input);
                let cmd_paragraph =
                    Paragraph::new(cmd_line).style(Style::default().fg(colors.info));
                frame.render_widget(cmd_paragraph, cmd_area);
                frame.render_widget(block, area);

                let cursor_x = area.x + 2 + self.vim_command_input.len() as u16;
                let cursor_y = area.bottom() - 2;
                if cursor_x < area.right() && cursor_y < area.bottom() {
                    frame.set_cursor_position(ratatui::layout::Position {
                        x: cursor_x,
                        y: cursor_y,
                    });
                }
            }
        } else {
            let paragraph = Paragraph::new("Select a key to view its value")
                .block(block)
                .wrap(Wrap { trim: true })
                .style(Style::default().fg(colors.text_secondary));
            frame.render_widget(paragraph, area);
        }
    }
}
