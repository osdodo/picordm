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

/// Filter the cache to avoid recalculating the filtering results on every render.
struct FilterCache {
    // Filtering mode for caching
    pattern: String,
    // Index of cached filtered results (index pointing to the original keys)
    indices: Vec<usize>,
    // Length of keys used to verify cache validity
    keys_len: usize,
}

impl FilterCache {
    fn new() -> Self {
        Self {
            pattern: String::new(),
            indices: Vec::new(),
            keys_len: 0,
        }
    }

    /// Check if the cache is valid
    fn is_valid(&self, pattern: &str, keys_len: usize) -> bool {
        self.pattern == pattern && self.keys_len == keys_len
    }

    /// Update cache
    fn update(&mut self, pattern: &str, keys: &[String]) {
        self.pattern = pattern.to_string();
        self.keys_len = keys.len();

        if pattern.is_empty() {
            // Without filtering, the index is 0..len
            self.indices = (0..keys.len()).collect();
        } else {
            let pattern_lower = pattern.to_lowercase();
            self.indices = keys
                .iter()
                .enumerate()
                .filter(|(_, key)| key.to_lowercase().contains(&pattern_lower))
                .map(|(i, _)| i)
                .collect();
        }
    }
}

pub struct KeyList {
    pub state: ListState,
    pub keys: Vec<String>,
    pub selected_keys: HashSet<String>,
    pub filter_pattern: String,
    pub is_loading: bool,
    pub is_focused: bool,
    filter_cache: FilterCache,
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
            filter_cache: FilterCache::new(),
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
                // The cache will be automatically updated the next time ensure_filter_cache is called.
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
                // Force update cache
                self.ensure_filter_cache();
                if !self.filter_cache.indices.is_empty() {
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

        // Ensure cache is valid
        self.ensure_filter_cache();

        // Use cached indexes to directly reference keys
        let items: Vec<ListItem> = self
            .filter_cache
            .indices
            .iter()
            .filter_map(|&idx| self.keys.get(idx))
            .map(|key| {
                let is_selected = self.selected_keys.contains(key);
                let checkbox = if is_selected {
                    Span::styled(
                        "[✓] ",
                        Style::default()
                            .fg(colors.text_key_selected)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled("[ ] ", Style::default().fg(colors.text_secondary))
                };

                let key_style = if is_selected {
                    Style::default().fg(colors.text_key_selected)
                } else {
                    Style::default().fg(colors.text_primary)
                };

                ListItem::new(Line::from(vec![
                    checkbox,
                    Span::styled(key.as_str(), key_style),
                ]))
            })
            .collect();

        let filtered_count = self.filter_cache.indices.len();
        let selected_count = self.selected_keys.len();
        let keys_title = if selected_count > 0 {
            format!("Keys ({}) - {} selected", filtered_count, selected_count)
        } else {
            format!("Keys ({})", filtered_count)
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

    /// Ensure the filter cache is valid; if invalid, update it.
    fn ensure_filter_cache(&mut self) {
        if !self
            .filter_cache
            .is_valid(&self.filter_pattern, self.keys.len())
        {
            self.filter_cache.update(&self.filter_pattern, &self.keys);
        }
    }

    /// Retrieve filtered key references.
    fn get_filtered_key_at(&self, index: usize) -> Option<&String> {
        self.filter_cache
            .indices
            .get(index)
            .and_then(|&idx| self.keys.get(idx))
    }

    fn next(&mut self) {
        self.ensure_filter_cache();
        let filtered_len = self.filter_cache.indices.len();
        if filtered_len == 0 {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= filtered_len - 1 {
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
        self.ensure_filter_cache();
        let filtered_len = self.filter_cache.indices.len();
        if filtered_len == 0 {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    filtered_len - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn selected_key(&self) -> Option<&String> {
        self.state
            .selected()
            .and_then(|i| self.filter_cache.indices.get(i))
            .and_then(|&idx| self.keys.get(idx))
    }

    fn toggle_selection(&mut self) {
        if let Some(selected_idx) = self.state.selected() {
            let key_to_toggle = self
                .filter_cache
                .indices
                .get(selected_idx)
                .and_then(|&idx| self.keys.get(idx))
                .cloned();

            if let Some(key) = key_to_toggle {
                if self.selected_keys.contains(&key) {
                    self.selected_keys.remove(&key);
                } else {
                    self.selected_keys.insert(key);
                }
            }
        }
    }

    fn select_all(&mut self) {
        self.ensure_filter_cache();
        let filtered_keys: Vec<&String> = self
            .filter_cache
            .indices
            .iter()
            .filter_map(|&idx| self.keys.get(idx))
            .collect();

        // Check if all filtered keys are already selected
        let all_selected = !filtered_keys.is_empty()
            && filtered_keys
                .iter()
                .all(|key| self.selected_keys.contains(*key));

        if all_selected {
            // Deselect all filtered keys
            for key in &filtered_keys {
                self.selected_keys.remove(*key);
            }
        } else {
            // Select all filtered keys
            for key in filtered_keys {
                self.selected_keys.insert(key.clone());
            }
        }
    }

    pub fn get_selected_key(&mut self) -> Option<String> {
        self.ensure_filter_cache();
        self.state
            .selected()
            .and_then(|i| self.get_filtered_key_at(i).cloned())
    }

    pub fn get_selected_keys(&mut self) -> Vec<String> {
        if self.selected_keys.is_empty() {
            self.get_selected_key().into_iter().collect()
        } else {
            self.selected_keys.iter().cloned().collect()
        }
    }
}
