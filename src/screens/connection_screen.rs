use anyhow::Result;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Direction, Layout},
    widgets::Clear,
};

use crate::models::{ConnectionConfig, Screen};
use crate::screens::utils::{centered_rect_fixed_size, render_background};
use crate::service::get_redis_service;
use crate::widgets::{
    connection_form, connection_list,
    footer::{self, Footer},
    header::{self, Header},
};

#[derive(Debug, Clone)]
pub enum Message {
    Form(connection_form::Message),
    List(Box<connection_list::Message>),
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

        if let Some(msg) = self.connection_list.handle_key_events(key) {
            return Some(Message::List(Box::new(msg)));
        }

        match (key.code, key.modifiers) {
            (KeyCode::Char('n'), _) => Some(Message::OpenNewForm),
            (KeyCode::Char('i'), _) => Some(Message::QuickImport),
            _ => None,
        }
    }

    pub async fn update(
        &mut self,
        msg: Message,
        terminal: &mut DefaultTerminal,
    ) -> Result<UpdateResult> {
        match msg {
            Message::Form(form_msg) => {
                if let connection_form::UpdateResult::Saved(config) =
                    self.connection_form.update(form_msg)
                {
                    let is_editing = self.connection_form.editing_connection_id.is_some();
                    let list_msg = if is_editing {
                        connection_list::Message::Update(*config)
                    } else {
                        connection_list::Message::Add(*config)
                    };

                    match self.connection_list.update(list_msg) {
                        connection_list::UpdateResult::SaveError(e) => {
                            self.footer.update(footer::Message::Error(Some(e)));
                        }
                        _ => {
                            self.footer.update(footer::Message::Error(None));
                        }
                    }
                }
                Ok(UpdateResult::Continue)
            }
            Message::List(list_msg) => match self.connection_list.update(*list_msg) {
                connection_list::UpdateResult::Selected(config) => {
                    self.attempt_connection(&config, terminal).await
                }
                connection_list::UpdateResult::Edit(config) => {
                    self.connection_form
                        .update(connection_form::Message::OpenEdit(Box::new(config)));
                    Ok(UpdateResult::Continue)
                }
                connection_list::UpdateResult::SaveError(error) => {
                    self.footer.update(footer::Message::Error(Some(error)));
                    Ok(UpdateResult::Continue)
                }
                connection_list::UpdateResult::None => Ok(UpdateResult::Continue),
            },
            Message::OpenNewForm => {
                self.connection_form
                    .update(connection_form::Message::OpenNew);
                Ok(UpdateResult::Continue)
            }
            Message::QuickImport => self.quick_import_from_clipboard(terminal).await,
        }
    }

    pub fn view(&mut self, frame: &mut Frame) {
        let area = frame.area();

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
        self.connection_list.view(frame, chunks[1]);
        self.footer.view(frame, chunks[2]);

        // If the form is open, render the form
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

    async fn attempt_connection(
        &mut self,
        config: &ConnectionConfig,
        terminal: &mut DefaultTerminal,
    ) -> Result<UpdateResult> {
        // Set the connection name to be connected, set the loading status, and clear error messages.
        self.header
            .update(header::Message::UpdateConnectionName(Some(
                config.name.clone(),
            )));
        self.header.update(header::Message::SetConnecting(true));
        self.footer.update(footer::Message::Error(None));
        terminal.draw(|frame| {
            self.view(frame);
        })?;

        match get_redis_service().connect(config).await {
            Ok(_) => {
                self.header.update(header::Message::SetConnecting(false));
                Ok(UpdateResult::SwitchScreen(
                    Screen::Dashboard,
                    Box::new(config.clone()),
                ))
            }
            Err(e) => {
                self.header.update(header::Message::SetConnecting(false));
                self.header
                    .update(header::Message::UpdateConnectionName(None));
                self.footer.update(footer::Message::Error(Some(format!(
                    "Failed to connect: {}",
                    e
                ))));
                Ok(UpdateResult::Continue)
            }
        }
    }

    async fn quick_import_from_clipboard(
        &mut self,
        terminal: &mut DefaultTerminal,
    ) -> Result<UpdateResult> {
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

        // Import connection from string
        let existing_names: Vec<String> = self
            .connection_list
            .connections
            .iter()
            .map(|c| c.name.clone())
            .collect();
        let new_conn = match connection_form::ConnectionForm::import_from_string(
            clipboard_text,
            &existing_names,
        ) {
            Ok(conn) => conn,
            Err(e) => {
                self.footer.update(footer::Message::Error(Some(e)));
                return Ok(UpdateResult::Continue);
            }
        };

        // Save to config
        match self
            .connection_list
            .update(connection_list::Message::Add(new_conn.clone()))
        {
            connection_list::UpdateResult::SaveError(e) => {
                self.footer.update(footer::Message::Error(Some(format!(
                    "Failed to save connection '{}': {}\nConnection was not saved.",
                    new_conn.name, e
                ))));
                Ok(UpdateResult::Continue)
            }
            _ => self.attempt_connection(&new_conn, terminal).await,
        }
    }
}
