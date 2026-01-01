use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent},
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
};

use crate::models::ConnectionConfig;
use crate::theme::get_colors;
use crate::widgets::connection_storage::{load_connections, save_connections};

#[derive(Debug, Clone)]
pub enum Message {
    Next,
    Previous,
    Select,
    Delete,
    Edit,
    Add(ConnectionConfig),
    Update(ConnectionConfig),
}

#[derive(Debug, Clone)]
pub enum UpdateResult {
    None,
    Selected(ConnectionConfig),
    Edit(ConnectionConfig),
    SaveError(String),
}

pub struct ConnectionList {
    pub connections: Vec<ConnectionConfig>,
    pub state: ListState,
}

impl ConnectionList {
    pub fn new() -> Self {
        let mut state = ListState::default();
        let connections = load_connections();
        if !connections.is_empty() {
            state.select(Some(0));
        }
        Self { connections, state }
    }

    pub fn handle_key_events(&self, key: KeyEvent) -> Option<Message> {
        match (key.code, key.modifiers) {
            (KeyCode::Char('j') | KeyCode::Down, _) => Some(Message::Next),
            (KeyCode::Char('k') | KeyCode::Up, _) => Some(Message::Previous),
            (KeyCode::Enter, _) => Some(Message::Select),
            (KeyCode::Backspace, _) | (KeyCode::Delete, _) => Some(Message::Delete),
            (KeyCode::Char('e'), _) => Some(Message::Edit),
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
                if let Some(config) = self.selected_connection().cloned() {
                    UpdateResult::Selected(config)
                } else {
                    UpdateResult::None
                }
            }
            Message::Delete => {
                self.delete_selected();
                match save_connections(&self.connections) {
                    Ok(_) => UpdateResult::None,
                    Err(e) => UpdateResult::SaveError(format!("Failed to save: {}", e)),
                }
            }
            Message::Edit => {
                if let Some(config) = self.selected_connection().cloned() {
                    UpdateResult::Edit(config)
                } else {
                    UpdateResult::None
                }
            }
            Message::Add(config) => match self.push(config) {
                Ok(_) => UpdateResult::None,
                Err(e) => UpdateResult::SaveError(e),
            },
            Message::Update(config) => match self.update_connection(config) {
                Ok(_) => UpdateResult::None,
                Err(e) => UpdateResult::SaveError(e),
            },
        }
    }

    pub fn view(&mut self, frame: &mut Frame, area: Rect) {
        let colors = get_colors();

        let items: Vec<ListItem> = self
            .connections
            .iter()
            .map(|conn| {
                let name_span = Span::styled(
                    &conn.name,
                    Style::default()
                        .fg(colors.text_primary)
                        .add_modifier(Modifier::BOLD),
                );

                let cluster_span = if conn.is_cluster {
                    Span::styled(" [Cluster]", Style::default().fg(colors.text_secondary))
                } else {
                    Span::raw("")
                };

                ListItem::new(Line::from(vec![name_span, cluster_span]))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(colors.border_active))
                    .title(Span::styled(
                        format!("Redis Connections ({})", self.connections.len()),
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

    fn next(&mut self) {
        if self.connections.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.connections.len() - 1 {
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
        if self.connections.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.connections.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn get_selected(&self) -> Option<&ConnectionConfig> {
        self.state.selected().and_then(|i| self.connections.get(i))
    }

    fn selected_connection(&self) -> Option<&ConnectionConfig> {
        self.get_selected()
    }

    fn push(&mut self, connection: ConnectionConfig) -> Result<(), String> {
        self.connections.push(connection);
        let new_index = self.connections.len() - 1;
        self.state.select(Some(new_index));

        save_connections(&self.connections).map_err(|e| format!("Failed to save: {}", e))
    }

    fn update_connection(&mut self, connection: ConnectionConfig) -> Result<(), String> {
        if let Some(pos) = self.connections.iter().position(|c| c.id == connection.id) {
            self.connections[pos] = connection;
        }
        save_connections(&self.connections).map_err(|e| format!("Failed to save: {}", e))
    }

    fn delete_selected(&mut self) {
        if let Some(selected) = self.state.selected() {
            self.connections.remove(selected);
            if !self.connections.is_empty() {
                let new_selected = selected.min(self.connections.len() - 1);
                self.state.select(Some(new_selected));
            } else {
                self.state.select(None);
            }
        }
    }
}
