use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
};

use crate::models::ConnectionConfig;
use crate::theme::get_colors;

#[derive(Debug, Clone)]
pub enum Message {
    Close,
    Next,
    Previous,
    Select,
    QuickSelect(usize),
    ToggleSearch,
    UpdateSearch(String),
}

pub struct ConnectionSwitcher {
    pub connections: Vec<ConnectionConfig>,
    pub state: ListState,
    pub search: String,
    pub is_search_mode: bool,
    pub current_connection_name: Option<String>,
    pub is_open: bool,
}

impl ConnectionSwitcher {
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
            state: ListState::default(),
            search: String::new(),
            is_search_mode: false,
            current_connection_name: None,
            is_open: false,
        }
    }

    pub fn handle_key_events(&self, key: KeyEvent) -> Option<Message> {
        if !self.is_open {
            return None;
        }
        match key.code {
            KeyCode::Esc => {
                if self.is_search_mode {
                    Some(Message::ToggleSearch)
                } else {
                    Some(Message::Close)
                }
            }
            KeyCode::Char('/') => Some(Message::ToggleSearch),
            KeyCode::Enter => Some(Message::Select),
            // Handle search mode input first, before navigation keys
            KeyCode::Char(c) if self.is_search_mode => {
                let text = format!("{}{}", self.search, c);
                Some(Message::UpdateSearch(text))
            }
            KeyCode::Backspace if self.is_search_mode => {
                let mut text = self.search.clone();
                text.pop();
                Some(Message::UpdateSearch(text))
            }
            // Navigation keys only work when not in search mode
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.is_search_mode {
                    Some(Message::Next)
                } else {
                    None
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.is_search_mode {
                    Some(Message::Previous)
                } else {
                    None
                }
            }
            // Quick select only works when not in search mode and search is empty
            KeyCode::Char(c)
                if c.is_ascii_digit() && !self.is_search_mode && self.search.is_empty() =>
            {
                if let Some(digit) = c.to_digit(10) {
                    let num = digit as usize;
                    if num > 0 && num <= self.connections.len().min(9) {
                        Some(Message::QuickSelect(num - 1))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn update(&mut self, msg: Message) -> Option<ConnectionConfig> {
        match msg {
            Message::Close => {
                self.is_open = false;
                self.is_search_mode = false;
                self.search.clear();
                None
            }
            Message::ToggleSearch => {
                self.is_search_mode = !self.is_search_mode;
                if !self.is_search_mode {
                    self.search.clear();
                }
                None
            }
            Message::Next => {
                self.next();
                None
            }
            Message::Previous => {
                self.previous();
                None
            }
            Message::Select => {
                self.is_open = false;
                self.is_search_mode = false;
                self.search.clear();
                self.get_selected().cloned()
            }
            Message::QuickSelect(index) => {
                self.is_open = false;
                self.is_search_mode = false;
                self.search.clear();
                self.state.select(Some(index));
                self.get_selected().cloned()
            }
            Message::UpdateSearch(text) => {
                self.search = text;
                // Auto-select first filtered result when searching
                let filtered = self.get_filtered_connections();
                if let Some((idx, _)) = filtered.first() {
                    self.state.select(Some(*idx));
                }
                None
            }
        }
    }

    pub fn view(&mut self, frame: &mut Frame, area: Rect) {
        if !self.is_open {
            return;
        }

        let total_connections = self.connections.len();
        let filtered_connections = self.get_filtered_connections();
        let is_filtering = !self.search.is_empty();

        let title = if is_filtering {
            format!(
                "Quick Connection Switch ({}/{} connections)",
                filtered_connections.len(),
                total_connections
            )
        } else {
            format!(
                "Quick Connection Switch ({} connections)",
                total_connections
            )
        };

        let block = Block::default()
            .title(Line::from(vec![Span::styled(
                title,
                Style::default()
                    .fg(get_colors().text_primary)
                    .add_modifier(Modifier::BOLD),
            )]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(get_colors().border_active))
            .style(Style::default().bg(get_colors().bg_dialog));
        frame.render_widget(block, area);

        let inner = area.inner(ratatui::layout::Margin {
            horizontal: 2,
            vertical: 1,
        });
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Search box
                Constraint::Min(1),    // List area
                Constraint::Length(1), // Help text
            ])
            .split(inner);

        // Render search box
        let search_text = if self.is_search_mode {
            if is_filtering {
                Span::styled(
                    format!("Search: {}", self.search),
                    Style::default().fg(get_colors().text_primary),
                )
            } else {
                Span::styled("Search: ", Style::default().fg(get_colors().text_secondary))
            }
        } else {
            Span::styled(
                "Press '/' to search...",
                Style::default().fg(get_colors().text_secondary),
            )
        };
        frame.render_widget(Paragraph::new(Line::from(vec![search_text])), chunks[0]);

        // Build connection list items
        let items: Vec<ListItem> = filtered_connections
            .iter()
            .map(|(original_idx, conn)| {
                let is_current = self
                    .current_connection_name
                    .as_ref()
                    .map(|name| name == &conn.name)
                    .unwrap_or(false);

                let name_style = if is_current {
                    Style::default()
                        .fg(get_colors().success)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(get_colors().text_primary)
                };

                let mut spans = vec![];

                // Number prefix
                if !is_filtering {
                    let num_str = format!("{:2} ", original_idx + 1);
                    let num_style = if *original_idx < 9 {
                        Style::default().fg(get_colors().text_secondary)
                    } else {
                        Style::default().fg(get_colors().text_inactive)
                    };
                    spans.push(Span::styled(num_str, num_style));
                } else {
                    spans.push(Span::styled("   ", Style::default()));
                }

                // Current connection indicator
                spans.push(if is_current {
                    Span::styled("● ", Style::default().fg(get_colors().success))
                } else {
                    Span::styled("  ", Style::default())
                });

                // Connection name with optional highlighting
                if is_filtering {
                    let filter_lower = self.search.to_lowercase();
                    let name_lower = conn.name.to_lowercase();

                    if let Some(pos) = name_lower.find(&filter_lower) {
                        spans.push(Span::styled(&conn.name[..pos], name_style));
                        spans.push(Span::styled(
                            &conn.name[pos..pos + self.search.len()],
                            Style::default()
                                .fg(get_colors().info)
                                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                        ));
                        spans.push(Span::styled(
                            &conn.name[pos + self.search.len()..],
                            name_style,
                        ));
                    } else {
                        spans.push(Span::styled(&conn.name, name_style));
                    }
                } else {
                    spans.push(Span::styled(&conn.name, name_style));
                }

                // Connection details
                spans.push(Span::styled(
                    format!(" ({}:{})", conn.host, conn.port),
                    Style::default().fg(get_colors().text_secondary),
                ));

                ListItem::new(Line::from(spans))
            })
            .collect();

        // Render list
        let list_widget = if items.is_empty() {
            List::new(vec![ListItem::new(Line::from(Span::styled(
                "No matching connections",
                Style::default().fg(get_colors().error),
            )))])
        } else {
            List::new(items)
                .highlight_style(
                    Style::default()
                        .bg(get_colors().bg_highlight)
                        .fg(get_colors().text_on_highlight)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ ")
        };

        let mut render_state = ListState::default();
        if let Some(selected_original_idx) = self.state.selected() {
            let filtered_pos = filtered_connections
                .iter()
                .position(|(idx, _)| *idx == selected_original_idx);
            render_state.select(filtered_pos);
        }

        frame.render_stateful_widget(list_widget, chunks[1], &mut render_state);

        // Help text
        self.render_help(frame, chunks[2], total_connections);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect, total: usize) {
        let purple = get_colors().accent;
        let gray = get_colors().text_secondary;
        let selected_idx = self.state.selected().unwrap_or(0);

        let help_spans = if self.is_search_mode {
            vec![
                Span::styled(
                    "↑↓",
                    Style::default().fg(purple).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Navigate  ", Style::default().fg(gray)),
                Span::styled(
                    "Backspace",
                    Style::default().fg(purple).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Clear  ", Style::default().fg(gray)),
                Span::styled(
                    "Enter",
                    Style::default().fg(purple).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Switch  ", Style::default().fg(gray)),
                Span::styled(
                    "Esc",
                    Style::default().fg(purple).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Exit Search", Style::default().fg(gray)),
            ]
        } else {
            vec![
                Span::styled(
                    "↑↓",
                    Style::default().fg(purple).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Navigate  ", Style::default().fg(gray)),
                Span::styled(
                    "1-9",
                    Style::default().fg(purple).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Quick  ", Style::default().fg(gray)),
                Span::styled(
                    "/",
                    Style::default().fg(purple).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Search  ", Style::default().fg(gray)),
                Span::styled(
                    "Enter",
                    Style::default().fg(purple).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Switch  ", Style::default().fg(gray)),
                Span::styled(
                    "Esc",
                    Style::default().fg(purple).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Cancel  ", Style::default().fg(gray)),
                Span::styled(
                    format!("[{}/{}]", selected_idx + 1, total),
                    Style::default().fg(get_colors().info),
                ),
            ]
        };

        frame.render_widget(
            Paragraph::new(Line::from(help_spans)).alignment(ratatui::layout::Alignment::Center),
            area,
        );
    }

    pub fn open(&mut self, connections: Vec<ConnectionConfig>, current: Option<String>) {
        self.connections = connections;
        self.current_connection_name = current.clone();
        self.search.clear();
        self.is_search_mode = false;
        self.is_open = true;

        if let Some(current_name) = current {
            if let Some(index) = self
                .connections
                .iter()
                .position(|conn| conn.name == current_name)
            {
                self.state.select(Some(index));
            } else {
                self.state.select(Some(0));
            }
        } else {
            self.state.select(Some(0));
        }
    }

    fn get_filtered_connections(&self) -> Vec<(usize, &ConnectionConfig)> {
        if self.search.is_empty() {
            self.connections.iter().enumerate().collect()
        } else {
            let input_lower = self.search.to_lowercase();
            self.connections
                .iter()
                .enumerate()
                .filter(|(_, conn)| {
                    conn.name.to_lowercase().contains(&input_lower)
                        || conn.host.to_lowercase().contains(&input_lower)
                })
                .collect()
        }
    }

    fn next(&mut self) {
        let filtered = self.get_filtered_connections();
        if filtered.is_empty() {
            return;
        }

        let current_selected = self.state.selected();
        let current_pos =
            current_selected.and_then(|sel| filtered.iter().position(|(idx, _)| *idx == sel));

        let next_pos = match current_pos {
            Some(pos) if pos < filtered.len() - 1 => pos + 1,
            _ => 0,
        };

        if let Some((original_idx, _)) = filtered.get(next_pos) {
            self.state.select(Some(*original_idx));
        }
    }

    fn previous(&mut self) {
        let filtered = self.get_filtered_connections();
        if filtered.is_empty() {
            return;
        }

        let current_selected = self.state.selected();
        let current_pos =
            current_selected.and_then(|sel| filtered.iter().position(|(idx, _)| *idx == sel));

        let prev_pos = match current_pos {
            Some(0) => filtered.len() - 1,
            Some(pos) => pos - 1,
            None => filtered.len() - 1,
        };

        if let Some((original_idx, _)) = filtered.get(prev_pos) {
            self.state.select(Some(*original_idx));
        }
    }

    fn get_selected(&self) -> Option<&ConnectionConfig> {
        self.state.selected().and_then(|i| self.connections.get(i))
    }
}
