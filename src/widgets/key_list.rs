use std::collections::HashSet;

use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
};

use crate::theme::get_colors;

#[derive(Debug, Clone)]
pub enum Message {
    Next,
    Previous,
    Select,
    ToggleSelection,
    SelectAll,
    ClearSelection,
    UpdateFilter(String),
    SetLoading(bool),
    UpdateKeys(Vec<String>),
    SetFocus(bool),
}

#[derive(Debug, Clone)]
pub enum UpdateResult {
    None,
    Selected(String),
}

pub struct KeyList {
    pub state: ListState,
    pub keys: Vec<String>,
    pub selected_keys: HashSet<String>,
    pub filter_pattern: String,
    pub is_loading: bool,
    pub is_focused: bool,
}

impl KeyList {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            state,
            keys: Vec::new(),
            selected_keys: HashSet::new(),
            filter_pattern: String::new(),
            is_loading: false,
            is_focused: false,
        }
    }

    pub fn handle_key_events(&self, key: KeyEvent) -> Option<Message> {
        match (key.code, key.modifiers) {
            (KeyCode::Char('j') | KeyCode::Down, _) => Some(Message::Next),
            (KeyCode::Char('k') | KeyCode::Up, _) => Some(Message::Previous),
            (KeyCode::Enter, _) => Some(Message::Select),
            (KeyCode::Char(' '), _) => Some(Message::ToggleSelection),
            (KeyCode::Char('a'), KeyModifiers::CONTROL) => Some(Message::SelectAll),
            _ => None,
        }
    }

    pub fn update(&mut self, msg: Message) -> UpdateResult {
        match msg {
            Message::Next => {
                self.next();
                UpdateResult::None
            }
            Message::Previous => {
                self.previous();
                UpdateResult::None
            }
            Message::Select => {
                if let Some(key) = self.selected_key().cloned() {
                    UpdateResult::Selected(key)
                } else {
                    UpdateResult::None
                }
            }
            Message::ToggleSelection => {
                self.toggle_selection();
                UpdateResult::None
            }
            Message::SelectAll => {
                self.select_all();
                UpdateResult::None
            }
            Message::ClearSelection => {
                self.selected_keys.clear();
                UpdateResult::None
            }
            Message::UpdateFilter(pattern) => {
                self.filter_pattern = pattern;
                UpdateResult::None
            }
            Message::SetLoading(loading) => {
                self.is_loading = loading;
                UpdateResult::None
            }
            Message::UpdateKeys(keys) => {
                self.keys = keys;
                self.selected_keys.clear();
                self.is_loading = false;
                if !self.get_filtered_keys().is_empty() {
                    self.state.select(Some(0));
                } else {
                    self.state.select(None);
                }
                UpdateResult::None
            }
            Message::SetFocus(focused) => {
                self.is_focused = focused;
                UpdateResult::None
            }
        }
    }

    pub fn view(&mut self, frame: &mut Frame, area: Rect) {
        let colors = get_colors();

        let border_color = if self.is_focused {
            colors.border_active
        } else {
            colors.border_default
        };

        if self.is_loading {
            let loading_text = Span::styled("Loading keys...", Style::default().fg(colors.info));
            let loading_widget = Paragraph::new(loading_text)
                .alignment(ratatui::layout::Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(border_color))
                        .title(Span::styled(
                            "Keys",
                            Style::default()
                                .fg(colors.text_primary)
                                .add_modifier(Modifier::BOLD),
                        )),
                );
            frame.render_widget(loading_widget, area);
            return;
        }

        let filtered_keys = self.get_filtered_keys();
        let items: Vec<ListItem> = filtered_keys
            .iter()
            .map(|key| {
                let is_selected = self.selected_keys.contains(key);
                let checkbox = if is_selected {
                    Span::styled(
                        "[✓] ",
                        Style::default()
                            .fg(colors.success)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled("[ ] ", Style::default().fg(colors.text_secondary))
                };

                let key_style = if is_selected {
                    Style::default().fg(colors.success)
                } else {
                    Style::default().fg(colors.text_primary)
                };

                ListItem::new(Line::from(vec![checkbox, Span::styled(key, key_style)]))
            })
            .collect();

        let selected_count = self.selected_keys.len();
        let keys_title = if selected_count > 0 {
            format!(
                "Keys ({}) - {} selected",
                filtered_keys.len(),
                selected_count
            )
        } else {
            format!("Keys ({})", filtered_keys.len())
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(border_color))
                    .title(Span::styled(
                        keys_title,
                        Style::default()
                            .fg(colors.text_primary)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .highlight_style(
                Style::default()
                    .bg(colors.bg_highlight)
                    .fg(colors.text_on_highlight)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, area, &mut self.state);
    }

    fn get_filtered_keys(&self) -> Vec<String> {
        if self.filter_pattern.is_empty() {
            self.keys.clone()
        } else {
            self.keys
                .iter()
                .filter(|key| {
                    key.to_lowercase()
                        .contains(&self.filter_pattern.to_lowercase())
                })
                .cloned()
                .collect()
        }
    }

    fn next(&mut self) {
        if self.keys.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.keys.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        if self.keys.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.keys.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn selected_key(&self) -> Option<&String> {
        self.state.selected().and_then(|i| self.keys.get(i))
    }

    fn toggle_selection(&mut self) {
        if let Some(selected_idx) = self.state.selected() {
            let filtered_keys = self.get_filtered_keys();
            if let Some(key) = filtered_keys.get(selected_idx) {
                if self.selected_keys.contains(key) {
                    self.selected_keys.remove(key);
                } else {
                    self.selected_keys.insert(key.clone());
                }
            }
        }
    }

    fn select_all(&mut self) {
        let filtered_keys = self.get_filtered_keys();
        // Check if all filtered keys are already selected
        let all_selected = !filtered_keys.is_empty()
            && filtered_keys
                .iter()
                .all(|key| self.selected_keys.contains(key));

        if all_selected {
            // Deselect all filtered keys
            for key in &filtered_keys {
                self.selected_keys.remove(key);
            }
        } else {
            // Select all filtered keys
            for key in filtered_keys {
                self.selected_keys.insert(key);
            }
        }
    }

    pub fn get_selected_key(&self) -> Option<String> {
        self.state
            .selected()
            .and_then(|i| self.get_filtered_keys().get(i).cloned())
    }

    pub fn get_selected_keys(&self) -> Vec<String> {
        if self.selected_keys.is_empty() {
            self.get_selected_key().into_iter().collect()
        } else {
            self.selected_keys.iter().cloned().collect()
        }
    }
}
