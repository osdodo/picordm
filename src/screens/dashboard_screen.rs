use anyhow::Result;
use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Direction, Layout, Rect},
};

use crate::models::{DbInfo, Screen, ServerInfo, ViewMode};
use crate::screens::{impex, utils::centered_rect_fixed_height};
use crate::service::get_redis_service;
use crate::widgets::*;

#[derive(Debug, Clone)]
pub enum Message {
    KeyList(key_list::Message),
    SearchBox(search_box::Message),
    DbSelectorKey(KeyCode),
    ToggleDbSelector,
    DeleteDialog(delete_dialog::Message),
    ConnectionSwitcherKey(KeyEvent),
    FileSelector(file_selector::Message),
    KeyContentEditor(key_content_editor::Message),
    CommandMode(command_mode::Message),

    Disconnect,
    OpenConnectionSwitcher,
    RefreshKeys,
    RefreshServerInfo,
    ExportData,
    EnterCommandMode,
    ExitKeyContent,
    ExitCommandMode,
}

#[derive(Debug, Clone)]
pub enum Action {
    None,
    SwitchScreen(Screen),
    LoadKeyValue(String),
    ExecuteCommand(String),
    SwitchDatabase(u32),
    DeleteKeys(Vec<String>),
    ImportFromFile(String),
    SwitchConnection(crate::models::ConnectionConfig),
    RefreshKeys,
    RefreshServerInfo,
    ExportData,
    SaveKeyContent,
    SaveAndQuitKeyContent,
}

#[derive(Debug)]
pub enum ActionResult {
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
    pub delete_dialog: DeleteDialog,
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
            delete_dialog: DeleteDialog::new(),
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
            (KeyCode::Char('t'), true) => return Some(Message::OpenConnectionSwitcher),
            (KeyCode::F(5), _) => return Some(Message::RefreshServerInfo),
            _ => {}
        }

        if self.delete_dialog.is_open {
            if let Some(msg) = self.delete_dialog.handle_key_events(key) {
                return Some(Message::DeleteDialog(msg));
            }
            return None;
        }

        if self.connection_switcher.is_open {
            if self.connection_switcher.handle_key_events(key).is_some() {
                return Some(Message::ConnectionSwitcherKey(key));
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
            if let Some(key_code) = self.db_selector.handle_key_events(key) {
                return Some(Message::DbSelectorKey(key_code));
            }
            return None;
        }

        // Handle key events based on view mode
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
                    (KeyCode::Char('n'), KeyModifiers::CONTROL) => Some(Message::ToggleDbSelector),
                    (KeyCode::Backspace | KeyCode::Delete, _) => {
                        Some(Message::DeleteDialog(delete_dialog::Message::Confirm))
                    }
                    (KeyCode::Char('e'), KeyModifiers::CONTROL) => Some(Message::ExportData),
                    (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
                        Some(Message::FileSelector(file_selector::Message::Show))
                    }
                    (KeyCode::Char('r'), KeyModifiers::CONTROL) => Some(Message::RefreshKeys),
                    (KeyCode::Char('>'), _) => Some(Message::EnterCommandMode),
                    _ => None,
                }
            }
            ViewMode::KeyContent => {
                if let Some(msg) = self.key_content_editor.handle_key_events(key) {
                    Some(Message::KeyContentEditor(msg))
                } else {
                    // Esc in Normal mode will return None, exit KeyContent
                    if key.code == KeyCode::Esc {
                        Some(Message::ExitKeyContent)
                    } else {
                        None
                    }
                }
            }
            ViewMode::CommandMode => {
                // Esc return to list
                if key.code == KeyCode::Esc {
                    // If in output area and not in Normal mode, pass to widget first
                    if self.command_mode.focus_on_output
                        && self.command_mode.editor_state.mode != edtui::EditorMode::Normal
                    {
                        if let Some(msg) = self.command_mode.handle_key_events(key) {
                            Some(Message::CommandMode(msg))
                        } else {
                            Some(Message::ExitCommandMode)
                        }
                    } else {
                        Some(Message::ExitCommandMode)
                    }
                } else {
                    self.command_mode
                        .handle_key_events(key)
                        .map(Message::CommandMode)
                }
            }
        }
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::KeyList(list_msg) => match list_msg {
                key_list::Message::Select => {
                    if let Some(key) = self.key_list.update(list_msg) {
                        Action::LoadKeyValue(key)
                    } else {
                        Action::None
                    }
                }
                _ => {
                    self.key_list.update(list_msg);
                    Action::None
                }
            },
            Message::SearchBox(search_msg) => {
                match search_msg.clone() {
                    search_box::Message::UpdateText(text) => {
                        self.search_box.update(search_msg);
                        self.key_list.update_filter(&text);
                    }
                    _ => {
                        self.search_box.update(search_msg);
                    }
                }
                Action::None
            }
            Message::ToggleDbSelector => {
                self.db_selector.toggle();
                Action::None
            }
            Message::DbSelectorKey(key_code) => match key_code {
                KeyCode::Esc => {
                    self.db_selector.toggle();
                    Action::None
                }
                KeyCode::Down => {
                    self.db_selector.next();
                    Action::None
                }
                KeyCode::Up => {
                    self.db_selector.previous();
                    Action::None
                }
                KeyCode::Enter => {
                    if let Some(selected) = self.db_selector.state.selected() {
                        if let Some(db_info) = self.db_selector.db_list.get(selected) {
                            let db_index = db_info.index;
                            self.db_selector.toggle();
                            self.db_selector.current_db_index = db_index;
                            Action::SwitchDatabase(db_index)
                        } else {
                            Action::None
                        }
                    } else {
                        Action::None
                    }
                }
                _ => Action::None,
            },
            Message::DeleteDialog(dialog_msg) => {
                match dialog_msg {
                    delete_dialog::Message::Confirm => {
                        // If dialog is not open, open it first
                        if !self.delete_dialog.is_open {
                            let keys_to_delete = self.key_list.get_selected_keys();
                            if !keys_to_delete.is_empty() {
                                self.delete_dialog.open(keys_to_delete.len());
                                Action::None
                            } else {
                                Action::None
                            }
                        } else {
                            // Dialog is open, handle confirm
                            self.delete_dialog.update(dialog_msg);
                            Action::DeleteKeys(self.key_list.get_selected_keys())
                        }
                    }
                    delete_dialog::Message::Cancel => {
                        self.delete_dialog.update(dialog_msg);
                        Action::None
                    }
                }
            }
            Message::ConnectionSwitcherKey(key) => {
                if let Some(msg) = self.connection_switcher.handle_key_events(key) {
                    if let Some(config) = self.connection_switcher.update(msg) {
                        Action::SwitchConnection(config)
                    } else {
                        Action::None
                    }
                } else {
                    Action::None
                }
            }
            Message::FileSelector(file_msg) => match file_msg {
                file_selector::Message::Enter => {
                    if let Some(path) = self.file_selector.update(file_msg) {
                        Action::ImportFromFile(path.to_string_lossy().to_string())
                    } else {
                        Action::None
                    }
                }
                _ => {
                    self.file_selector.update(file_msg);
                    Action::None
                }
            },
            Message::KeyContentEditor(editor_msg) => {
                // KeyContentEditor update returns Action, needs to be handled in handle_action
                let action = self.key_content_editor.update(editor_msg);
                match action {
                    key_content_editor::Action::Save => Action::SaveKeyContent,
                    key_content_editor::Action::SaveAndQuit => Action::SaveAndQuitKeyContent,
                    key_content_editor::Action::Quit => {
                        self.view_mode = ViewMode::KeyList;
                        Action::None
                    }
                    _ => Action::None,
                }
            }
            Message::CommandMode(cmd_msg) => match self.command_mode.update(cmd_msg.clone()) {
                command_mode::Action::ExecuteCommand { command } => Action::ExecuteCommand(command),
                command_mode::Action::None => Action::None,
            },
            Message::Disconnect => Action::SwitchScreen(Screen::Connection),
            Message::OpenConnectionSwitcher => {
                let connections = connection_storage::load_connections();
                self.connection_switcher
                    .open(connections, self.header.connection_name.as_ref().cloned());
                Action::None
            }
            Message::RefreshKeys => Action::RefreshKeys,
            Message::RefreshServerInfo => Action::RefreshServerInfo,
            Message::ExportData => Action::ExportData,
            Message::EnterCommandMode => {
                self.view_mode = ViewMode::CommandMode;
                self.command_mode.reset();
                Action::None
            }
            Message::ExitKeyContent => {
                self.view_mode = ViewMode::KeyList;
                Action::None
            }
            Message::ExitCommandMode => {
                self.view_mode = ViewMode::KeyList;
                Action::None
            }
        }
    }

    pub async fn handle_action(
        &mut self,
        action: Action,
        terminal: &mut ratatui::DefaultTerminal,
    ) -> Result<ActionResult> {
        match action {
            Action::SwitchScreen(screen) => {
                self.disconnect().await;
                Ok(ActionResult::SwitchScreen(screen))
            }
            Action::LoadKeyValue(key) => {
                self.view_mode = ViewMode::KeyContent;
                self.key_content_editor.set_loading_value(true);

                terminal.draw(|frame| {
                    let area = frame.area();
                    self.view(frame, area);
                })?;

                let db_index = self.get_current_db_index();
                match get_redis_service().get_value(&key, db_index).await {
                    Ok(value) => {
                        self.key_content_editor.load_key_value(key, value);
                    }
                    Err(e) => {
                        self.footer.update(footer::Message::Error(Some(format!(
                            "Failed to fetch value: {}",
                            e
                        ))));
                        self.key_content_editor.set_loading_value(false);
                    }
                }
                Ok(ActionResult::Continue)
            }
            Action::ExecuteCommand(command) => {
                let db_index = self.get_current_db_index();
                self.command_mode
                    .handle_action(
                        command_mode::Action::ExecuteCommand { command },
                        db_index,
                        |error| {
                            self.footer.update(footer::Message::Error(error));
                        },
                    )
                    .await?;
                Ok(ActionResult::Continue)
            }
            // Handle KeyContentEditor actions that need Redis context
            // These are handled inline when KeyContentEditorMessage is processed
            // because the editor state is already updated in the update() method
            Action::SwitchDatabase(db_index) => {
                self.set_loading_keys(true);

                terminal.draw(|frame| {
                    let area = frame.area();
                    self.view(frame, area);
                })?;

                let pattern = if self.search_box.text.is_empty() {
                    "*".to_string()
                } else {
                    self.search_box.text.clone()
                };

                match get_redis_service().get_keys(&pattern, db_index).await {
                    Ok(keys) => {
                        self.key_list.update_keys(keys);
                    }
                    Err(e) => {
                        self.footer.update(footer::Message::Error(Some(format!(
                            "Failed to fetch keys: {}",
                            e
                        ))));
                        self.set_loading_keys(false);
                    }
                }
                Ok(ActionResult::Continue)
            }
            Action::DeleteKeys(keys_to_delete) => {
                if !keys_to_delete.is_empty() {
                    let db_index = self.get_current_db_index();

                    self.progress_dialog.update(progress_dialog::Message::Show(
                        "Deleting Keys".to_string(),
                        format!("Deleting {} keys...", keys_to_delete.len()),
                    ));

                    terminal.draw(|frame| {
                        let area = frame.area();
                        self.view(frame, area);
                    })?;

                    for key in &keys_to_delete {
                        let _ = get_redis_service().delete_key(key, db_index).await;
                    }

                    self.key_list.clear_selection();
                    self.delete_dialog.close();

                    // Refresh server information and database list
                    if let Ok((info, db_list)) = get_redis_service().get_server_info().await {
                        self.update_server_info(Some(info));
                        self.update_db_list(db_list);
                    }

                    // Refresh list
                    let pattern = if self.search_box.text.is_empty() {
                        "*".to_string()
                    } else {
                        self.search_box.text.clone()
                    };

                    if let Ok(keys) = get_redis_service().get_keys(&pattern, db_index).await {
                        self.key_list.update_keys(keys);
                    }

                    self.progress_dialog.update(progress_dialog::Message::Hide);
                }
                Ok(ActionResult::Continue)
            }
            Action::ImportFromFile(path) => {
                self.file_selector.close();

                // Show progress dialog
                self.progress_dialog.update(progress_dialog::Message::Show(
                    "Importing Data".to_string(),
                    "Reading file...".to_string(),
                ));

                // Draw progress dialog
                terminal.draw(|frame| {
                    let area = frame.area();
                    self.view(frame, area);
                })?;

                // Execute import
                let db_index = self.get_current_db_index();
                match impex::import_redis_data(std::path::Path::new(&path), db_index, false).await {
                    Ok((imported, _skipped)) => {
                        self.progress_dialog
                            .update(progress_dialog::Message::Complete(format!(
                                "Successfully imported {} keys",
                                imported
                            )));

                        // Draw completion message
                        terminal.draw(|frame| {
                            let area = frame.area();
                            self.view(frame, area);
                        })?;

                        self.footer.update(footer::Message::Error(None));

                        // Refresh server information and database list
                        if let Ok((info, db_list)) = get_redis_service().get_server_info().await {
                            self.update_server_info(Some(info));
                            self.update_db_list(db_list);
                        }

                        // Refresh list
                        let pattern = if self.search_box.text.is_empty() {
                            "*".to_string()
                        } else {
                            self.search_box.text.clone()
                        };

                        if let Ok(keys) = get_redis_service().get_keys(&pattern, db_index).await {
                            self.key_list.update_keys(keys);
                        }

                        // Wait for a moment to let the user see the completion message
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
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
                Ok(ActionResult::Continue)
            }
            Action::SwitchConnection(config) => {
                self.connection_switcher.is_open = false;

                // Disconnect current connection
                self.disconnect().await;

                // Connect new Redis
                self.set_loading_keys(true);
                terminal.draw(|frame| {
                    let area = frame.area();
                    self.view(frame, area);
                })?;

                match self.connect_and_load(&config, terminal).await {
                    Ok(_) => Ok(ActionResult::Continue),
                    Err(e) => {
                        self.set_loading_keys(false);
                        // Show error message and stay on dashboard screen
                        self.footer.update(footer::Message::Error(Some(format!(
                            "Failed to connect: {}",
                            e
                        ))));
                        Ok(ActionResult::Continue)
                    }
                }
            }
            Action::RefreshKeys => {
                self.set_loading_keys(true);

                terminal.draw(|frame| {
                    let area = frame.area();
                    self.view(frame, area);
                })?;

                let pattern = if self.search_box.text.is_empty() {
                    "*".to_string()
                } else {
                    self.search_box.text.clone()
                };

                let db_index = self.get_current_db_index();
                match get_redis_service().get_keys(&pattern, db_index).await {
                    Ok(keys) => {
                        self.key_list.update_keys(keys);
                    }
                    Err(e) => {
                        self.footer.update(footer::Message::Error(Some(format!(
                            "Failed to fetch keys: {}",
                            e
                        ))));
                        self.set_loading_keys(false);
                    }
                }
                Ok(ActionResult::Continue)
            }
            Action::RefreshServerInfo => {
                self.set_loading_server_info(true);

                terminal.draw(|frame| {
                    let area = frame.area();
                    self.view(frame, area);
                })?;

                match get_redis_service().get_server_info().await {
                    Ok((info, db_list)) => {
                        self.update_server_info(Some(info));
                        self.update_db_list(db_list);
                    }
                    Err(e) => {
                        self.footer.update(footer::Message::Error(Some(format!(
                            "Failed to fetch server info: {}",
                            e
                        ))));
                    }
                }

                self.set_loading_server_info(false);
                Ok(ActionResult::Continue)
            }
            Action::ExportData => {
                self.export_data(terminal).await?;
                Ok(ActionResult::Continue)
            }
            Action::SaveKeyContent => {
                if let Some(key) = self.key_content_editor.get_current_key() {
                    let value = self.key_content_editor.get_editor_content();
                    let db_index = self.get_current_db_index();

                    match get_redis_service().set_value(&key, &value, db_index).await {
                        Ok(_) => {
                            self.key_content_editor.content = value;
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
                Ok(ActionResult::Continue)
            }
            Action::SaveAndQuitKeyContent => {
                if let Some(key) = self.key_content_editor.get_current_key() {
                    let value = self.key_content_editor.get_editor_content();
                    let db_index = self.get_current_db_index();
                    let _ = get_redis_service().set_value(&key, &value, db_index).await;
                }
                self.view_mode = ViewMode::KeyList;
                Ok(ActionResult::Continue)
            }
            _ => Ok(ActionResult::Continue),
        }
    }

    pub fn view(&mut self, frame: &mut Frame, area: Rect) {
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

        self.header.view(frame, chunks[0], false);
        self.footer.view(frame, chunks[2]);

        match self.view_mode {
            ViewMode::KeyList => self.render_key_list_view(frame, chunks[1]),
            ViewMode::KeyContent => self.render_key_content_view(frame, chunks[1]),
            ViewMode::CommandMode => self.render_command_mode_view(frame, chunks[1]),
        }

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
        self.render_content_area(frame, main_chunks[1]);
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

    fn render_dialogs(&mut self, frame: &mut Frame, area: Rect) {
        if self.delete_dialog.is_open {
            let popup_area = centered_rect_fixed_height(60, 8, area);
            frame.render_widget(ratatui::widgets::Clear, popup_area);
            self.delete_dialog.view(frame, popup_area);
        }

        if self.progress_dialog.is_visible {
            let popup_area = centered_rect_fixed_height(60, 8, area);
            frame.render_widget(ratatui::widgets::Clear, popup_area);
            self.progress_dialog.view(frame, popup_area);
        }

        if self.connection_switcher.is_open {
            let connections_count = self.connection_switcher.connections.len();
            let list_height = (connections_count as u16).min(12);
            let popup_height = list_height + 5;
            let popup_area = centered_rect_fixed_height(70, popup_height, area);
            frame.render_widget(ratatui::widgets::Clear, popup_area);
            self.connection_switcher.view(frame, popup_area);
        }

        if self.file_selector.is_open {
            let popup_area = centered_rect_fixed_height(70, 18, area);
            frame.render_widget(ratatui::widgets::Clear, popup_area);
            self.file_selector.view(frame, popup_area);
        }
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

        // Only focus on key list in KeyList view mode
        let key_list_focused = self.view_mode == ViewMode::KeyList && !self.search_box.is_focused;
        self.key_list.view(frame, chunks[1], key_list_focused);
        self.db_selector.view(frame, chunks[2]);

        if let Some((x, y)) = cursor_pos {
            frame.set_cursor_position(ratatui::layout::Position { x, y });
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

        let paragraph =
            Paragraph::new("Select a key to view its value or press '>' to execute a command")
                .block(block)
                .wrap(Wrap { trim: true })
                .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(paragraph, area);
    }

    fn update_server_info(&mut self, info: Option<ServerInfo>) {
        self.header.update(header::Message::UpdateServerInfo(info));
    }

    fn update_db_list(&mut self, db_list: Vec<DbInfo>) {
        self.db_selector.update_db_list(db_list);
    }

    fn set_loading_keys(&mut self, loading: bool) {
        self.key_list.is_loading = loading;
    }

    fn set_loading_server_info(&mut self, loading: bool) {
        self.header
            .update(header::Message::SetLoadingServerInfo(loading));
    }

    async fn connect_and_load(
        &mut self,
        config: &crate::models::ConnectionConfig,
        terminal: &mut ratatui::DefaultTerminal,
    ) -> Result<()> {
        get_redis_service().connect(config).await?;
        self.load_data(config, terminal).await
    }

    pub async fn load_data(
        &mut self,
        config: &crate::models::ConnectionConfig,
        terminal: &mut ratatui::DefaultTerminal,
    ) -> Result<()> {
        self.header
            .update(header::Message::UpdateConnectionName(Some(
                config.name.clone(),
            )));
        self.footer.update(footer::Message::Error(None));

        // Set loading state
        self.set_loading_server_info(true);
        self.set_loading_keys(true);

        // Draw loading state
        terminal.draw(|frame| {
            let area = frame.area();
            self.view(frame, area);
        })?;

        // Load server information and database list
        if let Ok((info, db_list)) = get_redis_service().get_server_info().await {
            self.update_server_info(Some(info));
            self.update_db_list(db_list);
        }

        // Load keys
        let db_index = self.get_current_db_index();
        if let Ok(keys) = get_redis_service().get_keys("*", db_index).await {
            self.key_list.update_keys(keys);
        }

        Ok(())
    }

    pub async fn disconnect(&mut self) {
        get_redis_service().disconnect().await;

        // Reset state
        self.view_mode = ViewMode::KeyList;
        self.key_list.update_keys(vec![]);
        self.header.update(header::Message::UpdateServerInfo(None));
        self.header
            .update(header::Message::UpdateConnectionName(None));
    }

    pub fn get_current_db_index(&self) -> u32 {
        self.db_selector.current_db_index
    }

    async fn export_data(&mut self, terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
        // Check if there is a connection
        if self.header.connection_name.is_none() {
            self.footer.update(footer::Message::Error(Some(
                "No Redis connection active".to_string(),
            )));
            return Ok(());
        }

        // Get keys to export
        let keys = if self.key_list.selected_keys.is_empty() {
            // If no keys are selected, export all keys
            self.key_list.keys.clone()
        } else {
            // Export selected keys
            self.key_list.selected_keys.iter().cloned().collect()
        };

        if keys.is_empty() {
            self.footer.update(footer::Message::Error(Some(
                "No keys to export".to_string(),
            )));
            return Ok(());
        }

        let connection_name = self.header.connection_name.clone().unwrap_or_default();
        let database = self.get_current_db_index();
        self.export_keys(terminal, connection_name, database, keys)
            .await
    }

    async fn export_keys(
        &mut self,
        terminal: &mut ratatui::DefaultTerminal,
        connection_name: String,
        database: u32,
        keys: Vec<String>,
    ) -> Result<()> {
        // Show progress dialog
        self.progress_dialog.update(progress_dialog::Message::Show(
            "Export Data".to_string(),
            format!("Exporting {} keys...", keys.len()),
        ));

        // Draw progress dialog
        terminal.draw(|frame| {
            let area = frame.area();
            self.view(frame, area);
        })?;

        // Generate file name
        let file_name = format!(
            "redis_data_{}_{}_db{}_{}.json",
            connection_name.replace(' ', "_"),
            chrono::Local::now().format("%Y%m%d_%H%M%S"),
            database,
            keys.len()
        );

        let file_path = super::impex::get_export_path(&file_name);

        // Execute export
        match super::impex::export_redis_data(connection_name, database, &keys, &file_path).await {
            Ok(exported_count) => {
                let location = if file_path.starts_with(dirs::desktop_dir().unwrap_or_default()) {
                    "Desktop"
                } else {
                    "current directory"
                };

                self.progress_dialog
                    .update(progress_dialog::Message::Complete(format!(
                        "Exported {} keys to {}",
                        exported_count, location
                    )));

                // Draw completion message
                terminal.draw(|frame| {
                    let area = frame.area();
                    self.view(frame, area);
                })?;

                self.footer.update(footer::Message::Error(None));

                // Wait for a moment to let the user see the completion message
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
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
