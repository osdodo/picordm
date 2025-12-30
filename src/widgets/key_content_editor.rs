use edtui::{
    EditorEventHandler, EditorMode, EditorState, EditorTheme, EditorView, Lines, SyntaxHighlighter,
};
use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

#[derive(Debug, Clone)]
pub enum Message {
    EditorKeyEvent(KeyEvent),
    VimCommandModeToggle,
    VimCommandModeUpdateInput(String),
    VimCommandModeExecute,
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
    pub content: String,
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
            content: String::new(),
            is_json: false,
            is_vim_command_mode: false,
            vim_command_input: String::new(),
            is_loading_value: false,
        }
    }

    pub fn handle_key_events(&self, key: KeyEvent) -> Option<Message> {
        if self.is_vim_command_mode {
            return self.handle_vim_command_key(key);
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
                    // Normal mode, Esc exits editor (handled by parent).
                    return None;
                }
            }
        }

        Some(Message::EditorKeyEvent(key))
    }

    fn handle_vim_command_key(&self, key: KeyEvent) -> Option<Message> {
        match key.code {
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
        }
    }

    pub fn update(&mut self, msg: Message) -> UpdateResult {
        match msg {
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
                    "wq" | "x" => UpdateResult::SaveAndQuit,
                    "q!" => UpdateResult::Quit,
                    _ => UpdateResult::None,
                };

                self.is_vim_command_mode = false;
                self.vim_command_input.clear();

                result
            }
        }
    }

    pub fn view(&mut self, frame: &mut Frame, area: Rect) {
        let title = if self.is_vim_command_mode {
            "Vim Command Mode (Enter to execute, Esc to cancel)".to_string()
        } else if !self.content.is_empty() {
            let mode_hint = match self.editor_state.mode {
                EditorMode::Normal => "Press ':' for commands (:w=Save :q=Quit :wq=Save&Quit)",
                EditorMode::Insert => "Press Esc to Normal mode, then ':' for commands",
                EditorMode::Visual => "Press Esc to Normal mode, then ':' for commands",
                EditorMode::Search => "Press Esc to Normal mode, then ':' for commands",
            };
            format!("View/Edit ({})", mode_hint)
        } else {
            "View".to_string()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(147, 112, 219)))
            .title(Span::styled(
                title,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ));

        if self.is_loading_value {
            let loading_text = Span::styled("Loading value...", Style::default().fg(Color::Yellow));
            let paragraph = Paragraph::new(loading_text)
                .block(block)
                .alignment(ratatui::layout::Alignment::Center);
            frame.render_widget(paragraph, area);
        } else if !self.content.is_empty() {
            if self.is_vim_command_mode {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1), Constraint::Length(1)])
                    .split(area.inner(Margin::new(1, 1)));

                let mut editor_view = EditorView::new(&mut self.editor_state);

                if self.is_json {
                    let syntax_highlighter = SyntaxHighlighter::new("visual-studio-dark", "json");
                    editor_view = editor_view.syntax_highlighter(Some(syntax_highlighter));
                }

                frame.render_widget(editor_view, chunks[0]);

                let cmd_line = format!(":{}", self.vim_command_input);
                let cmd_paragraph =
                    Paragraph::new(cmd_line).style(Style::default().fg(Color::Yellow));
                frame.render_widget(cmd_paragraph, chunks[1]);

                frame.render_widget(block, area);

                let cursor_x = area.x + 2 + self.vim_command_input.len() as u16;
                let cursor_y = area.bottom() - 2;
                if cursor_x < area.right() && cursor_y < area.bottom() {
                    frame.set_cursor_position(ratatui::layout::Position {
                        x: cursor_x,
                        y: cursor_y,
                    });
                }
            } else {
                let theme = EditorTheme {
                    block: Some(block),
                    ..Default::default()
                };

                let mut editor_view = EditorView::new(&mut self.editor_state).theme(theme);

                if self.is_json {
                    let syntax_highlighter = SyntaxHighlighter::new("visual-studio-dark", "json");
                    editor_view = editor_view.syntax_highlighter(Some(syntax_highlighter));
                }

                frame.render_widget(editor_view, area);
            }
        } else {
            let paragraph = Paragraph::new("Select a key to view its value")
                .block(block)
                .wrap(Wrap { trim: true })
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(paragraph, area);
        }
    }

    pub fn load_key_value(&mut self, key: String, value: String) {
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

        self.content = content.clone();
        self.is_json = is_json;
        self.editor_state = EditorState::new(Lines::from(content.as_str()));
        self.is_loading_value = false;
    }

    pub fn get_editor_content(&self) -> String {
        String::from(self.editor_state.lines.clone())
    }

    pub fn set_loading_value(&mut self, loading: bool) {
        self.is_loading_value = loading;
    }

    pub fn get_current_key(&self) -> Option<String> {
        self.current_key.clone()
    }
}
