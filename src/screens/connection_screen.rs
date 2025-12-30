use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Direction, Layout, Rect},
};

use crate::models::{ConnectionConfig, Screen};
use crate::screens::utils::centered_rect_fixed_height;
use crate::service::get_redis_service;
use crate::widgets::*;

#[derive(Debug, Clone)]
pub enum Message {
    Form(connection_form::Message),
    List(connection_list::Message),
    OpenNewForm,
    QuickImport,
}

#[derive(Debug, Clone)]
pub enum Action {
    None,
    SwitchScreen(Screen, Box<ConnectionConfig>),
}

#[derive(Debug)]
pub enum ActionResult {
    Continue,
    SwitchScreen(Screen, Box<ConnectionConfig>),
}

pub struct ConnectionScreen {
    pub connection_list: connection_list::ConnectionList,
    pub connection_form: ConnectionForm,
    pub header: Header,
    pub footer: Footer,
    pub is_connecting: bool,
}

impl ConnectionScreen {
    pub fn new() -> Self {
        let mut footer = Footer::new();
        footer.update(footer::Message::Screen(Screen::Connection));

        Self {
            connection_list: ConnectionList::new(),
            connection_form: ConnectionForm::new(),
            header: Header::new(),
            footer,
            is_connecting: false,
        }
    }

    pub fn handle_key_events(&self, key: KeyEvent) -> Option<Message> {
        // If the form is open, form events should be handled first.
        if self.connection_form.is_open
            && let Some(msg) = self.connection_form.handle_key_events(key)
        {
            return Some(Message::Form(msg));
        }

        // Handling list events
        if let Some(msg) = self.connection_list.handle_key_events(key) {
            return Some(Message::List(msg));
        }

        // Other shortcut keys
        match (key.code, key.modifiers) {
            (KeyCode::Char('n'), _) => Some(Message::OpenNewForm),
            (KeyCode::Char('i'), _) => Some(Message::QuickImport),
            _ => None,
        }
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::Form(form_msg) => match self.connection_form.update(form_msg) {
                connection_form::FormAction::Saved(config) => {
                    self.save_connection(*config);
                    Action::None
                }
                connection_form::FormAction::Cancelled => {
                    Action::None
                }
                _ => Action::None,
            },
            Message::List(list_msg) => match self.connection_list.update(list_msg) {
                connection_list::Action::Selected(config) => {
                    Action::SwitchScreen(Screen::Dashboard, Box::new(config))
                }
                connection_list::Action::Edit(config) => {
                    self.connection_form.open_edit(&config);
                    Action::None
                }
                connection_list::Action::SaveError(error) => {
                    self.footer.update(footer::Message::Error(Some(error)));
                    Action::None
                }
                connection_list::Action::None => Action::None,
            },
            Message::OpenNewForm => {
                self.connection_form.open_new();
                Action::None
            }
            Message::QuickImport => self.quick_import_from_clipboard(),
        }
    }

    pub async fn handle_action(
        &mut self,
        action: Action,
        terminal: &mut ratatui::DefaultTerminal,
    ) -> anyhow::Result<ActionResult> {
        match action {
            Action::SwitchScreen(screen, config) => {
                self.is_connecting = true;
                self.footer.update(footer::Message::Error(None));
                terminal.draw(|frame| {
                    let area = frame.area();
                    self.view(frame, area);
                })?;

                match get_redis_service().connect(&config).await {
                    Ok(_) => {
                        self.is_connecting = false;
                        Ok(ActionResult::SwitchScreen(screen, config))
                    }
                    Err(e) => {
                        self.is_connecting = false;
                        self.footer.update(footer::Message::Error(Some(format!(
                            "Failed to connect: {}",
                            e
                        ))));
                        Ok(ActionResult::Continue)
                    }
                }
            }
            _ => Ok(ActionResult::Continue),
        }
    }

    pub fn view(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Footer
            ])
            .split(area);

        self.header.view(frame, chunks[0], self.is_connecting);
        self.footer.view(frame, chunks[2]);

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30), // Left side: Connection list
                Constraint::Percentage(70), // Right side: Content area
            ])
            .split(chunks[1]);

        self.connection_list.view(frame, main_chunks[0]);
        self.render_content_area(frame, main_chunks[1]);

        // If the form is open, render the form (on top of it)
        if self.connection_form.is_open {
            let form_height = if self.connection_form.validation_error.is_some() {
                30
            } else {
                27
            };
            let form_area = centered_rect_fixed_height(60, form_height, area);
            frame.render_widget(ratatui::widgets::Clear, form_area);
            self.connection_form.view(frame, form_area);
        }
    }

    fn save_connection(&mut self, config: ConnectionConfig) {
        let is_editing = self.connection_form.editing_connection_id.is_some();
        let result = if is_editing {
            self.connection_list.update_connection(config)
        } else {
            self.connection_list.push(config)
        };

        match result {
            Ok(_) => self.footer.update(footer::Message::Error(None)),
            Err(e) => self.footer.update(footer::Message::Error(Some(e))),
        }
    }

    fn quick_import_from_clipboard(&mut self) -> Action {
        let clipboard_text = match arboard::Clipboard::new() {
            Ok(mut clipboard) => match clipboard.get_text() {
                Ok(text) => text,
                Err(e) => {
                    self.footer.update(footer::Message::Error(Some(format!(
                        "Failed to read clipboard: {}",
                        e
                    ))));
                    return Action::None;
                }
            },
            Err(e) => {
                self.footer.update(footer::Message::Error(Some(format!(
                    "Failed to access clipboard: {}",
                    e
                ))));
                return Action::None;
            }
        };

        let clipboard_text = clipboard_text.trim();
        if clipboard_text.is_empty() {
            self.footer.update(footer::Message::Error(Some(
                "Clipboard is empty".to_string(),
            )));
            return Action::None;
        }

        // Parse connection string
        let mut form = match ConnectionForm::from_connection_string(clipboard_text) {
            Ok(form) => form,
            Err(e) => {
                self.footer.update(footer::Message::Error(Some(e)));
                return Action::None;
            }
        };

        // Generate name if empty
        if form.name.is_empty() {
            if form.is_cluster {
                let first_node = form
                    .cluster_nodes
                    .split(',')
                    .next()
                    .unwrap_or("cluster")
                    .trim()
                    .to_string();

                // Extract host from first node (remove port)
                let base_name = if let Some(idx) = first_node.find(':') {
                    &first_node[..idx]
                } else {
                    &first_node
                };

                let name_exists = self.connection_list.connections.iter().any(|c| {
                    c.name == base_name || c.name.starts_with(&format!("{}-cluster", base_name))
                });
                form.name = if name_exists {
                    format!(
                        "{}-cluster-{}",
                        base_name,
                        chrono::Local::now().format("%H%M%S")
                    )
                } else {
                    format!("{}-cluster", base_name)
                };
            } else {
                let host_exists = self
                    .connection_list
                    .connections
                    .iter()
                    .any(|c| c.name == form.host);
                form.name = if host_exists {
                    format!("{}-{}", form.host, chrono::Local::now().format("%H%M%S"))
                } else {
                    form.host.clone()
                };
            }
        }

        // Parse db_aliases
        let db_aliases: std::collections::HashMap<u32, String> =
            if form.db_aliases.trim().is_empty() {
                std::collections::HashMap::new()
            } else {
                serde_json::from_str(&form.db_aliases).unwrap_or_default()
            };

        // Create connection config
        let new_conn = form.to_connection_config(db_aliases);

        // Save to config
        match self.connection_list.push(new_conn.clone()) {
            Ok(_) => {
                self.footer.update(footer::Message::Error(None));
                // Auto-connect after import
                Action::SwitchScreen(Screen::Dashboard, Box::new(new_conn))
            }
            Err(e) => {
                self.footer.update(footer::Message::Error(Some(format!(
                    "Failed to save connection '{}': {}\nConnection was not saved.",
                    new_conn.name, e
                ))));
                Action::None
            }
        }
    }

    fn render_content_area(&self, frame: &mut Frame, area: Rect) {
        use ratatui::style::{Color, Modifier, Style};
        use ratatui::text::Span;
        use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(80, 90, 110)))
            .title(Span::styled(
                "View",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ));

        // Show error message or default prompt
        let content = if let Some(err) = &self.footer.error_message {
            Span::styled(format!("Error: {}", err), Style::default().fg(Color::Red))
        } else {
            Span::styled(
                "Select a connection to start",
                Style::default().fg(Color::White),
            )
        };

        let paragraph = Paragraph::new(content)
            .block(block)
            .wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);
    }
}
