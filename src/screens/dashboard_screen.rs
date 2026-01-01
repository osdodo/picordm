use anyhow::Result;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};
use tokio::{
    sync::mpsc,
    time::{Duration, sleep},
};

use crate::impex;
use crate::models::{ConnectionConfig, Screen, ViewMode};
use crate::screens::utils::{centered_rect_fixed_size, render_background};
use crate::service::get_redis_service;
use crate::theme::get_colors;
use crate::widgets::{
    command_mode::{self, CommandMode},
    confirm_dialog::{self, ConfirmDialog},
    connection_switcher::{self, ConnectionSwitcher},
    db_selector::{self, DbSelector},
    file_selector::{self, FileSelector},
    footer::{self, Footer},
    header::{self, Header},
    key_content_editor::{self, KeyContentEditor},
    key_list::{self, KeyList},
    progress_dialog::{self, ProgressDialog},
    search_box::{self, SearchBox},
};

#[derive(Debug, Clone)]
pub enum Message {
    LoadData(ConnectionConfig),
    KeyList(key_list::Message),
    KeyContentEditor(key_content_editor::Message),
    SearchBox(search_box::Message),
    ConfirmDialog(confirm_dialog::Message),
    RequestDeleteConfirmation,
    DbSelector(db_selector::Message),
    Disconnect,
    ConnectionSwitcher(connection_switcher::Message),
    RefreshKeys,
    RefreshServerInfo,
    ExportData,
    FileSelector(file_selector::Message),
    CommandMode(command_mode::Message),
}

#[derive(Debug)]
pub enum UpdateResult {
    Continue,
    SwitchScreen(Screen),
}

pub struct DashboardScreen {
    pub view_mode: ViewMode,

    pub header: Header,
    pub footer: Footer,
    pub key_list: KeyList,
    pub search_box: SearchBox,
    pub db_selector: DbSelector,
    pub confirm_dialog: ConfirmDialog,
    pub progress_dialog: ProgressDialog,
    pub connection_switcher: ConnectionSwitcher,
    pub file_selector: FileSelector,
    pub key_content_editor: KeyContentEditor,
    pub command_mode: CommandMode,
}

impl DashboardScreen {
    pub fn new() -> Self {
        let mut footer = Footer::new();
        footer.update(footer::Message::Screen(Screen::Dashboard));

        Self {
            view_mode: ViewMode::KeyList,
            header: Header::new(),
            footer,
            key_list: KeyList::new(),
            search_box: SearchBox::new("Search Keys"),
            db_selector: DbSelector::new(),
            confirm_dialog: ConfirmDialog::new(),
            progress_dialog: ProgressDialog::new(),
            connection_switcher: ConnectionSwitcher::new(),
            file_selector: FileSelector::new(),
            key_content_editor: KeyContentEditor::new(),
            command_mode: CommandMode::new(),
        }
    }

    pub fn handle_key_events(&self, key: KeyEvent) -> Option<Message> {
        match (key.code, key.modifiers.contains(KeyModifiers::CONTROL)) {
            (KeyCode::Char('b'), true) => return Some(Message::Disconnect),
            (KeyCode::Char('t'), true) => {
                return Some(Message::ConnectionSwitcher(
                    connection_switcher::Message::Show(
                        self.header.connection_name.as_ref().cloned(),
                    ),
                ));
            }
            (KeyCode::F(5), _) => return Some(Message::RefreshServerInfo),
            _ => {}
        }

        if self.confirm_dialog.is_open {
            if let Some(msg) = self.confirm_dialog.handle_key_events(key) {
                return Some(Message::ConfirmDialog(msg));
            }
            return None;
        }

        if self.connection_switcher.is_open {
            if let Some(msg) = self.connection_switcher.handle_key_events(key) {
                return Some(Message::ConnectionSwitcher(msg));
            }
            return None;
        }

        if self.file_selector.is_open {
            if let Some(msg) = self.file_selector.handle_key_events(key) {
                return Some(Message::FileSelector(msg));
            }
            return None;
        }

        if self.db_selector.is_open {
            if let Some(msg) = self.db_selector.handle_key_events(key) {
                return Some(Message::DbSelector(msg));
            }
            return None;
        }

        match self.view_mode {
            ViewMode::KeyList => {
                // Search box focus
                if self.search_box.is_focused
                    && let Some(msg) = self.search_box.handle_key_events(key)
                {
                    return Some(Message::SearchBox(msg));
                }

                // Handle list keys
                if let Some(msg) = self.key_list.handle_key_events(key) {
                    return Some(Message::KeyList(msg));
                }

                // Handle other shortcut keys
                match (key.code, key.modifiers) {
                    (KeyCode::Char('/'), _) => {
                        Some(Message::SearchBox(search_box::Message::ToggleFocus))
                    }
                    (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                        Some(Message::DbSelector(db_selector::Message::Toggle))
                    }
                    (KeyCode::Backspace | KeyCode::Delete, _) => {
                        Some(Message::RequestDeleteConfirmation)
                    }
                    (KeyCode::Char('e'), KeyModifiers::CONTROL) => Some(Message::ExportData),
                    (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
                        Some(Message::FileSelector(file_selector::Message::Show))
                    }
                    (KeyCode::Char('r'), KeyModifiers::CONTROL) => Some(Message::RefreshKeys),
                    (KeyCode::Char('>'), _) => Some(Message::CommandMode(
                        command_mode::Message::Enter(self.db_selector.current_db_index),
                    )),
                    _ => None,
                }
            }
            ViewMode::KeyContent => self
                .key_content_editor
                .handle_key_events(key)
                .map(Message::KeyContentEditor),
            ViewMode::CommandMode => self
                .command_mode
                .handle_key_events(key)
                .map(Message::CommandMode),
        }
    }

    pub async fn update(
        &mut self,
        msg: Message,
        terminal: &mut DefaultTerminal,
    ) -> Result<UpdateResult> {
        match msg {
            Message::LoadData(config) => {
                self.header
                    .update(header::Message::UpdateConnectionName(Some(
                        config.name.clone(),
                    )));
                if let Err(e) = self.load_data(terminal).await {
                    self.footer.update(footer::Message::Error(Some(format!(
                        "Failed to load dashboard: {}",
                        e
                    ))));
                }
                Ok(UpdateResult::Continue)
            }
            Message::KeyList(msg) => match msg {
                key_list::Message::Select => {
                    if let key_list::UpdateResult::Selected(key) = self.key_list.update(msg) {
                        self.view_mode = ViewMode::KeyContent;
                        self.key_list.update(key_list::Message::SetFocus(false));
                        self.key_content_editor
                            .update(key_content_editor::Message::SetLoadingValue(true));
                        terminal.draw(|frame| {
                            self.view(frame);
                        })?;

                        let db_index = self.db_selector.current_db_index;
                        match get_redis_service().get_value(&key, db_index).await {
                            Ok(value) => {
                                self.key_content_editor
                                    .update(key_content_editor::Message::LoadKeyValue(key, value));
                            }
                            Err(e) => {
                                self.footer.update(footer::Message::Error(Some(format!(
                                    "Failed to fetch value: {}",
                                    e
                                ))));
                                self.key_content_editor
                                    .update(key_content_editor::Message::SetLoadingValue(false));
                            }
                        }
                    }

                    Ok(UpdateResult::Continue)
                }
                _ => {
                    self.key_list.update(msg);
                    Ok(UpdateResult::Continue)
                }
            },
            Message::SearchBox(msg) => {
                let was_focused = self.search_box.is_focused;
                match self.search_box.update(msg) {
                    search_box::UpdateResult::TextUpdated(text) => {
                        self.key_list.update(key_list::Message::UpdateFilter(text));
                    }
                    search_box::UpdateResult::None => {}
                }
                // Update key_list focus based on search_box focus state
                if self.view_mode == ViewMode::KeyList {
                    let should_focus = !self.search_box.is_focused;
                    if was_focused != self.search_box.is_focused {
                        self.key_list
                            .update(key_list::Message::SetFocus(should_focus));
                    }
                }
                Ok(UpdateResult::Continue)
            }
            Message::DbSelector(msg) => match self.db_selector.update(msg) {
                db_selector::UpdateResult::Selected(db_index) => {
                    self.refresh_keys(terminal, db_index).await;
                    Ok(UpdateResult::Continue)
                }
                db_selector::UpdateResult::None => Ok(UpdateResult::Continue),
            },
            Message::RequestDeleteConfirmation => {
                let keys_to_delete = self.key_list.get_selected_keys();
                if !keys_to_delete.is_empty() {
                    let count = keys_to_delete.len();
                    let message = format!(
                        "Are you sure you want to delete {} key{}?",
                        count,
                        if count == 1 { "" } else { "s" }
                    );
                    self.confirm_dialog.update(confirm_dialog::Message::Show {
                        title: "Confirm Delete".to_string(),
                        message,
                    });
                }
                Ok(UpdateResult::Continue)
            }
            Message::ConfirmDialog(msg) => match msg {
                confirm_dialog::Message::Confirm => {
                    self.confirm_dialog.update(msg);
                    let keys_to_delete = self.key_list.get_selected_keys();
                    if !keys_to_delete.is_empty() {
                        let db_index = self.db_selector.current_db_index;

                        self.progress_dialog.update(progress_dialog::Message::Show(
                            "Deleting Keys".to_string(),
                            format!("Deleting {} keys...", keys_to_delete.len()),
                        ));
                        terminal.draw(|frame| {
                            self.view(frame);
                        })?;

                        for key in &keys_to_delete {
                            let _ = get_redis_service().delete_key(key, db_index).await;
                        }

                        self.key_list.update(key_list::Message::ClearSelection);
                        self.confirm_dialog.update(confirm_dialog::Message::Cancel);

                        self.refresh_data(db_index).await;
                        self.progress_dialog.update(progress_dialog::Message::Hide);
                    }
                    Ok(UpdateResult::Continue)
                }
                confirm_dialog::Message::Cancel => {
                    self.confirm_dialog.update(msg);
                    Ok(UpdateResult::Continue)
                }
                _ => Ok(UpdateResult::Continue),
            },
            Message::ConnectionSwitcher(msg) => {
                match self.connection_switcher.update(msg) {
                    connection_switcher::UpdateResult::Selected(config) => {
                        self.connection_switcher
                            .update(connection_switcher::Message::SetOpen(false));

                        // Disconnect current connection
                        self.disconnect().await;

                        // Set connecting state and update connection name
                        self.header
                            .update(header::Message::UpdateConnectionName(Some(
                                config.name.clone(),
                            )));
                        self.header.update(header::Message::SetConnecting(true));
                        self.footer.update(footer::Message::Error(None));
                        terminal.draw(|frame| {
                            self.view(frame);
                        })?;

                        match get_redis_service().connect(&config).await {
                            Ok(_) => {
                                self.header.update(header::Message::SetConnecting(false));
                                if let Err(e) = self.load_data(terminal).await {
                                    self.footer.update(footer::Message::Error(Some(format!(
                                        "Failed to load data: {}",
                                        e
                                    ))));
                                }
                                Ok(UpdateResult::Continue)
                            }
                            Err(e) => {
                                self.header
                                    .update(header::Message::UpdateConnectionName(None));
                                self.header.update(header::Message::SetConnecting(false));
                                self.footer.update(footer::Message::Error(Some(format!(
                                    "Failed to connect: {}",
                                    e
                                ))));
                                Ok(UpdateResult::Continue)
                            }
                        }
                    }
                    connection_switcher::UpdateResult::None => Ok(UpdateResult::Continue),
                }
            }
            Message::FileSelector(msg) => match self.file_selector.update(msg) {
                file_selector::UpdateResult::Selected(path) => {
                    self.file_selector.update(file_selector::Message::Close);
                    self.progress_dialog
                        .update(progress_dialog::Message::ShowWithProgress(
                            "Importing Data".to_string(),
                            "Reading file...".to_string(),
                            0,
                        ));
                    terminal.draw(|frame| {
                        self.view(frame);
                    })?;

                    let db_index = self.db_selector.current_db_index;

                    // Use Tokio channels to receive progress updates.
                    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel();

                    // Start import task
                    let mut import_handle = {
                        let path = path.clone();

                        tokio::spawn(async move {
                            impex::import_redis_data(
                                &path,
                                db_index,
                                false,
                                Some(move |current, total| {
                                    let _ = progress_tx.send((current, total));
                                }),
                            )
                            .await
                        })
                    };

                    let result = loop {
                        tokio::select! {
                            progress_msg = progress_rx.recv() => {
                                match progress_msg {
                                    Some((current, total)) => {
                                        let mut latest_current = current;
                                        let mut latest_total = total;
                                        while let Ok((c, t)) = progress_rx.try_recv() {
                                            latest_current = c;
                                            latest_total = t;
                                        }

                                        self.progress_dialog.update(progress_dialog::Message::UpdateProgress(
                                            latest_current,
                                            latest_total,
                                            "Importing keys...".to_string(),
                                        ));
                                        terminal.draw(|frame| {
                                            self.view(frame);
                                        })?;
                                    }
                                    None => {
                                        // The channel is closed, indicating that the sender has dropped the request; continue waiting for the task to complete.
                                    }
                                }
                            }
                            task_result = &mut import_handle => {
                                break task_result.map_err(|e| anyhow::anyhow!("Import task failed: {}", e))?;
                            }
                        }
                    };

                    match result {
                        Ok(result) => {
                            let mut message =
                                format!("Successfully imported {} keys", result.imported_count);
                            if result.skipped_count > 0 {
                                message.push_str(&format!(", {} skipped", result.skipped_count));
                            }
                            if !result.failed_keys.is_empty() {
                                message.push_str(&format!(", {} failed", result.failed_keys.len()));
                            }
                            let errors = if !result.failed_keys.is_empty() {
                                Some(
                                    result
                                        .failed_keys
                                        .iter()
                                        .map(|(k, e)| format!("{}: {}", k, e))
                                        .collect(),
                                )
                            } else {
                                None
                            };

                            self.progress_dialog
                                .update(progress_dialog::Message::Complete(message, errors));
                            terminal.draw(|frame| {
                                self.view(frame);
                            })?;

                            self.refresh_data(db_index).await;
                            sleep(Duration::from_secs(2)).await;
                            self.progress_dialog.update(progress_dialog::Message::Hide);
                        }
                        Err(e) => {
                            self.progress_dialog.update(progress_dialog::Message::Hide);
                            self.footer.update(footer::Message::Error(Some(format!(
                                "Failed to import: {}",
                                e
                            ))));
                        }
                    }
                    Ok(UpdateResult::Continue)
                }
                file_selector::UpdateResult::None => Ok(UpdateResult::Continue),
            },
            Message::KeyContentEditor(msg) => match self.key_content_editor.update(msg) {
                key_content_editor::UpdateResult::Save => {
                    self.save_key_content().await;
                    Ok(UpdateResult::Continue)
                }
                key_content_editor::UpdateResult::SaveAndQuit => {
                    self.save_key_content().await;
                    self.view_mode = ViewMode::KeyList;
                    self.key_list
                        .update(key_list::Message::SetFocus(!self.search_box.is_focused));
                    Ok(UpdateResult::Continue)
                }
                key_content_editor::UpdateResult::Quit => {
                    self.view_mode = ViewMode::KeyList;
                    self.key_list
                        .update(key_list::Message::SetFocus(!self.search_box.is_focused));
                    Ok(UpdateResult::Continue)
                }
                _ => Ok(UpdateResult::Continue),
            },
            Message::CommandMode(msg) => match self.command_mode.update(msg).await? {
                command_mode::UpdateResult::EnterCommandMode => {
                    self.view_mode = ViewMode::CommandMode;
                    self.key_list.update(key_list::Message::SetFocus(false));
                    Ok(UpdateResult::Continue)
                }
                command_mode::UpdateResult::ExitCommandMode => {
                    self.view_mode = ViewMode::KeyList;
                    self.key_list
                        .update(key_list::Message::SetFocus(!self.search_box.is_focused));
                    Ok(UpdateResult::Continue)
                }
                command_mode::UpdateResult::Continue => Ok(UpdateResult::Continue),
            },
            Message::Disconnect => {
                self.disconnect().await;
                Ok(UpdateResult::SwitchScreen(Screen::Connection))
            }
            Message::RefreshKeys => {
                let db_index = self.db_selector.current_db_index;
                self.refresh_keys(terminal, db_index).await;
                Ok(UpdateResult::Continue)
            }
            Message::RefreshServerInfo => {
                self.header
                    .update(header::Message::SetLoadingServerInfo(true));
                terminal.draw(|frame| {
                    self.view(frame);
                })?;

                self.load_server_info().await;

                Ok(UpdateResult::Continue)
            }
            Message::ExportData => {
                self.export_data(terminal).await?;
                Ok(UpdateResult::Continue)
            }
        }
    }

    pub fn view(&mut self, frame: &mut Frame) {
        let area = frame.area();

        render_background(frame, area);

        self.footer
            .update(footer::Message::ViewMode(self.view_mode));

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Footer
            ])
            .split(area);

        self.header.view(frame, chunks[0]);
        match self.view_mode {
            ViewMode::KeyList => self.render_key_list_view(frame, chunks[1]),
            ViewMode::KeyContent => self.render_key_content_view(frame, chunks[1]),
            ViewMode::CommandMode => self.render_command_mode_view(frame, chunks[1]),
        }
        self.footer.view(frame, chunks[2]);
        self.render_dialogs(frame, area);
    }

    fn render_key_list_view(&mut self, frame: &mut Frame, area: Rect) {
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30), // Left: Key list
                Constraint::Percentage(70), // Right: Content area
            ])
            .split(area);

        self.render_sidebar(frame, main_chunks[0]);

        // Content area
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(get_colors().border_default))
            .title(Span::styled(
                "View",
                Style::default()
                    .fg(get_colors().text_primary)
                    .add_modifier(Modifier::BOLD),
            ));

        let paragraph =
            Paragraph::new("Select a key to view its value or press '>' to execute a command")
                .block(block)
                .wrap(Wrap { trim: true })
                .style(Style::default().fg(get_colors().text_secondary));
        frame.render_widget(paragraph, main_chunks[1]);
    }

    fn render_key_content_view(&mut self, frame: &mut Frame, area: Rect) {
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(area);

        self.render_sidebar(frame, main_chunks[0]);
        self.key_content_editor.view(frame, main_chunks[1]);
    }

    fn render_command_mode_view(&mut self, frame: &mut Frame, area: Rect) {
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(area);

        self.render_sidebar(frame, main_chunks[0]);
        self.command_mode.view(frame, main_chunks[1]);
    }

    fn render_sidebar(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search box
                Constraint::Min(1),    // Key list
                Constraint::Length(3), // DB selector
            ])
            .split(area);

        let cursor_pos = self.search_box.view(frame, chunks[0]);

        self.key_list.view(frame, chunks[1]);
        self.db_selector.view(frame, chunks[2]);

        if let Some((x, y)) = cursor_pos {
            frame.set_cursor_position(ratatui::layout::Position { x, y });
        }
    }

    fn render_dialogs(&mut self, frame: &mut Frame, area: Rect) {
        if self.confirm_dialog.is_open {
            let popup_area = centered_rect_fixed_size(70, 8, area);
            frame.render_widget(Clear, popup_area);
            self.confirm_dialog.view(frame, popup_area);
        }

        if self.progress_dialog.is_visible {
            let popup_area = centered_rect_fixed_size(90, 12, area);
            frame.render_widget(Clear, popup_area);
            self.progress_dialog.view(frame, popup_area);
        }

        if self.connection_switcher.is_open {
            let connections_count = self.connection_switcher.connections.len();
            let list_height = (connections_count as u16).min(12);
            let popup_height = list_height + 5;
            let popup_area = centered_rect_fixed_size(120, popup_height, area);
            frame.render_widget(Clear, popup_area);
            self.connection_switcher.view(frame, popup_area);
        }

        if self.file_selector.is_open {
            let popup_area = centered_rect_fixed_size(90, 18, area);
            frame.render_widget(Clear, popup_area);
            self.file_selector.view(frame, popup_area);
        }
    }

    fn get_search_pattern(&self) -> String {
        if self.search_box.text.is_empty() {
            "*".to_string()
        } else {
            self.search_box.text.clone()
        }
    }

    async fn load_server_info(&mut self) {
        if let Ok((info, db_list)) = get_redis_service().get_server_info().await {
            self.header
                .update(header::Message::UpdateServerInfo(Some(info)));
            self.db_selector
                .update(db_selector::Message::UpdateDbList(db_list));
        }
    }

    async fn load_keys(&mut self, pattern: &str, db_index: u32) -> Result<(), String> {
        get_redis_service()
            .get_keys(pattern, db_index)
            .await
            .map(|keys| {
                self.key_list.update(key_list::Message::UpdateKeys(keys));
            })
            .map_err(|e| format!("Failed to fetch keys: {}", e))
    }

    pub async fn load_data(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        self.header
            .update(header::Message::SetLoadingServerInfo(true));
        self.key_list.update(key_list::Message::SetLoading(true));
        terminal.draw(|frame| {
            self.view(frame);
        })?;

        self.load_server_info().await;

        let db_index = self.db_selector.current_db_index;
        self.load_keys("*", db_index).await.ok();

        // Set focus state after loading data
        if self.view_mode == ViewMode::KeyList {
            self.key_list
                .update(key_list::Message::SetFocus(!self.search_box.is_focused));
        }

        Ok(())
    }

    async fn refresh_keys(&mut self, terminal: &mut DefaultTerminal, db_index: u32) {
        self.key_list.update(key_list::Message::SetLoading(true));
        terminal
            .draw(|frame| {
                self.view(frame);
            })
            .ok();

        let pattern = self.get_search_pattern();
        if let Err(e) = self.load_keys(&pattern, db_index).await {
            self.footer.update(footer::Message::Error(Some(e)));
            self.key_list.update(key_list::Message::SetLoading(false));
        }
    }

    async fn refresh_data(&mut self, db_index: u32) {
        self.load_server_info().await;

        let pattern = self.get_search_pattern();
        self.load_keys(&pattern, db_index).await.ok();
    }

    pub async fn disconnect(&mut self) {
        get_redis_service().disconnect().await;

        // Reset state
        self.header.update(header::Message::UpdateServerInfo(None));
        self.header
            .update(header::Message::UpdateConnectionName(None));
        self.view_mode = ViewMode::KeyList;
        self.search_box
            .update(search_box::Message::UpdateText(String::new()));
        self.key_list
            .update(key_list::Message::UpdateFilter(String::new()));
        self.key_list.update(key_list::Message::UpdateKeys(vec![]));
        self.key_list.update(key_list::Message::SetFocus(false));
    }

    async fn save_key_content(&mut self) {
        if let Some(key) = self.key_content_editor.current_key.clone() {
            let value = String::from(self.key_content_editor.editor_state.lines.clone());
            let db_index = self.db_selector.current_db_index;

            match get_redis_service().set_value(&key, &value, db_index).await {
                Ok(_) => {
                    self.footer.update(footer::Message::Error(None));
                }
                Err(e) => {
                    self.footer.update(footer::Message::Error(Some(format!(
                        "Failed to save: {}",
                        e
                    ))));
                }
            }
        }
    }

    pub async fn export_data(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let connection_name = match &self.header.connection_name {
            Some(name) => name.clone(),
            None => {
                self.footer.update(footer::Message::Error(Some(
                    "No Redis connection active".to_string(),
                )));
                return Ok(());
            }
        };

        let keys: Vec<String> = if self.key_list.selected_keys.is_empty() {
            self.key_list.keys.clone()
        } else {
            self.key_list.selected_keys.iter().cloned().collect()
        };

        if keys.is_empty() {
            self.footer.update(footer::Message::Error(Some(
                "No keys to export".to_string(),
            )));
            return Ok(());
        }

        let database = self.db_selector.current_db_index;

        // Show progress dialog with initial progress
        self.progress_dialog
            .update(progress_dialog::Message::ShowWithProgress(
                "Export Data".to_string(),
                "Exporting keys...".to_string(),
                keys.len(),
            ));
        terminal.draw(|frame| {
            self.view(frame);
        })?;

        let file_name = format!(
            "redis_data_{}_{}_db{}_{}.json",
            connection_name.replace(' ', "_"),
            chrono::Local::now().format("%Y%m%d_%H%M%S"),
            database,
            keys.len()
        );
        let file_path = impex::get_export_path(&file_name);

        let (progress_tx, mut progress_rx) = mpsc::unbounded_channel();

        let mut export_task = {
            let connection_name = connection_name.clone();
            let keys = keys.clone();
            let file_path = file_path.clone();

            tokio::spawn(async move {
                impex::export_redis_data(
                    connection_name,
                    database,
                    &keys,
                    &file_path,
                    Some(move |current, total| {
                        let _ = progress_tx.send((current, total));
                    }),
                )
                .await
            })
        };

        let result = loop {
            tokio::select! {
                progress_msg = progress_rx.recv() => {
                    if let Some((current, total)) = progress_msg {
                        // Consume all pending progress updates in batches
                        let mut latest_current = current;
                        let mut latest_total = total;
                        while let Ok((c, t)) = progress_rx.try_recv() {
                            latest_current = c;
                            latest_total = t;
                        }

                       // Update the UI to show the latest progress
                        self.progress_dialog.update(progress_dialog::Message::UpdateProgress(
                            latest_current,
                            latest_total,
                            "Exporting keys...".to_string(),
                        ));
                        terminal.draw(|frame| {
                            self.view(frame);
                        })?;
                    }
                }
                task_result = &mut export_task => {
                    break task_result.map_err(|e| anyhow::anyhow!("Export task failed: {}", e))?;
                }
            }
        };

        match result {
            Ok(result) => {
                let location = if file_path.starts_with(dirs::desktop_dir().unwrap_or_default()) {
                    "Desktop"
                } else {
                    "current directory"
                };

                let mut message =
                    format!("Exported {} keys to {}", result.exported_count, location);
                if !result.failed_keys.is_empty() {
                    message.push_str(&format!(", {} failed", result.failed_keys.len()));
                }

                let error_list = (!result.failed_keys.is_empty()).then(|| {
                    result
                        .failed_keys
                        .iter()
                        .map(|(k, e)| format!("{}: {}", k, e))
                        .collect::<Vec<_>>()
                });

                self.progress_dialog
                    .update(progress_dialog::Message::Complete(message, error_list));
                terminal.draw(|frame| {
                    self.view(frame);
                })?;

                sleep(Duration::from_secs(2)).await;
                self.progress_dialog.update(progress_dialog::Message::Hide);
            }
            Err(e) => {
                self.progress_dialog.update(progress_dialog::Message::Hide);
                self.footer.update(footer::Message::Error(Some(format!(
                    "Export failed: {}",
                    e
                ))));
            }
        }

        Ok(())
    }
}
