use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

use crate::screens::utils::{centered_rect_fixed_size, render_background};
use crate::service::get_redis_service;
use crate::theme::get_colors;
use crate::widgets::{
    connection_form, connection_list,
    footer::{self, Footer},
    header::{self, Header},
};
use crate::{
    models::{ConnectionConfig, Screen},
    screens::utils::draw_with_background,
};

#[derive(Debug, Clone)]
pub enum Message {
    Form(connection_form::Message),
    List(connection_list::Message),
    OpenNewForm,
    QuickImport,
}

#[derive(Debug)]
pub enum UpdateResult {
    Continue,
    SwitchScreen(Screen, Box<ConnectionConfig>),
}

pub struct ConnectionScreen {
    pub connection_list: connection_list::ConnectionList,
    pub connection_form: connection_form::ConnectionForm,
    pub header: Header,
    pub footer: Footer,
}

impl ConnectionScreen {
    pub fn new() -> Self {
        let mut footer = Footer::new();
        footer.update(footer::Message::Screen(Screen::Connection));

        Self {
            connection_list: connection_list::ConnectionList::new(),
            connection_form: connection_form::ConnectionForm::new(),
            header: Header::new(),
            footer,
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

    pub async fn update(
        &mut self,
        msg: Message,
        terminal: &mut ratatui::DefaultTerminal,
    ) -> anyhow::Result<UpdateResult> {
        match msg {
            Message::Form(form_msg) => {
                match self.connection_form.update(form_msg) {
                    connection_form::UpdateResult::Saved(config) => {
                        self.save_connection(*config);
                    }
                    connection_form::UpdateResult::Cancelled => {}
                    _ => {}
                }
                Ok(UpdateResult::Continue)
            }
            Message::List(list_msg) => match self.connection_list.update(list_msg) {
                connection_list::UpdateResult::Selected(config) => {
                    self.header
                        .update(header::Message::UpdateConnectionName(Some(
                            config.name.clone(),
                        )));
                    self.header.update(header::Message::SetConnecting(true));
                    self.footer.update(footer::Message::Error(None));
                    draw_with_background(terminal, |frame| self.view(frame))?;

                    match get_redis_service().connect(&config).await {
                        Ok(_) => {
                            self.header.update(header::Message::SetConnecting(false));
                            Ok(UpdateResult::SwitchScreen(
                                Screen::Dashboard,
                                Box::new(config),
                            ))
                        }
                        Err(e) => {
                            self.header.update(header::Message::SetConnecting(false));
                            self.footer.update(footer::Message::Error(Some(format!(
                                "Failed to connect: {}",
                                e
                            ))));
                            Ok(UpdateResult::Continue)
                        }
                    }
                }
                connection_list::UpdateResult::Edit(config) => {
                    self.connection_form.open_edit(&config);
                    Ok(UpdateResult::Continue)
                }
                connection_list::UpdateResult::SaveError(error) => {
                    self.footer.update(footer::Message::Error(Some(error)));
                    Ok(UpdateResult::Continue)
                }
                connection_list::UpdateResult::None => Ok(UpdateResult::Continue),
            },
            Message::OpenNewForm => {
                self.connection_form.open_new();
                Ok(UpdateResult::Continue)
            }
            Message::QuickImport => self.quick_import_from_clipboard(terminal).await,
        }
    }

    pub fn view(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Set background color for the entire frame
        render_background(frame, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Footer
            ])
            .split(area);

        self.header.view(frame, chunks[0]);
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
            let form_area = centered_rect_fixed_size(120, form_height, area);
            frame.render_widget(Clear, form_area);
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

    async fn quick_import_from_clipboard(
        &mut self,
        terminal: &mut ratatui::DefaultTerminal,
    ) -> anyhow::Result<UpdateResult> {
        let clipboard_text = match arboard::Clipboard::new() {
            Ok(mut clipboard) => match clipboard.get_text() {
                Ok(text) => text,
                Err(e) => {
                    self.footer.update(footer::Message::Error(Some(format!(
                        "Failed to read clipboard: {}",
                        e
                    ))));
                    return Ok(UpdateResult::Continue);
                }
            },
            Err(e) => {
                self.footer.update(footer::Message::Error(Some(format!(
                    "Failed to access clipboard: {}",
                    e
                ))));
                return Ok(UpdateResult::Continue);
            }
        };

        let clipboard_text = clipboard_text.trim();
        if clipboard_text.is_empty() {
            self.footer.update(footer::Message::Error(Some(
                "Clipboard is empty".to_string(),
            )));
            return Ok(UpdateResult::Continue);
        }

        // Parse connection string
        let mut form = match connection_form::ConnectionForm::from_connection_string(clipboard_text)
        {
            Ok(form) => form,
            Err(e) => {
                self.footer.update(footer::Message::Error(Some(e)));
                return Ok(UpdateResult::Continue);
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
                self.header.update(header::Message::SetConnecting(true));
                self.footer.update(footer::Message::Error(None));
                draw_with_background(terminal, |frame| self.view(frame))?;

                match get_redis_service().connect(&new_conn).await {
                    Ok(_) => {
                        self.header.update(header::Message::SetConnecting(false));
                        Ok(UpdateResult::SwitchScreen(
                            Screen::Dashboard,
                            Box::new(new_conn),
                        ))
                    }
                    Err(e) => {
                        self.header.update(header::Message::SetConnecting(false));
                        self.footer.update(footer::Message::Error(Some(format!(
                            "Failed to connect: {}",
                            e
                        ))));
                        Ok(UpdateResult::Continue)
                    }
                }
            }
            Err(e) => {
                self.footer.update(footer::Message::Error(Some(format!(
                    "Failed to save connection '{}': {}\nConnection was not saved.",
                    new_conn.name, e
                ))));
                Ok(UpdateResult::Continue)
            }
        }
    }

    fn render_content_area(&self, frame: &mut Frame, area: Rect) {
        let colors = get_colors();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(colors.border_default))
            .title(Span::styled(
                "View",
                Style::default()
                    .fg(colors.text_primary)
                    .add_modifier(Modifier::BOLD),
            ));

        // Show error message or default prompt
        let content = if let Some(err) = &self.footer.error_message {
            Span::styled(format!("Error: {}", err), Style::default().fg(colors.error))
        } else {
            Span::styled(
                "Select a connection to start",
                Style::default().fg(colors.text_primary),
            )
        };

        let paragraph = Paragraph::new(content)
            .block(block)
            .wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);
    }
}
