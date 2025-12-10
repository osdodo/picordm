use anyhow::Result;
use ratatui::widgets::ListState;
use tui_textarea::TextArea;

use crate::connection::{ConnectionForm, ConnectionList};
use crate::service::{DbInfo, RedisService, ServerInfo};
use crate::storage::{load_connections, save_connections};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum CurrentScreen {
    ConnectionList,
    Dashboard,
    KeyContent,
    NewConnectionForm,
    JsonEditor,
    CommandMode,
}

pub struct App<'a> {
    // Screen state
    pub current_screen: CurrentScreen,

    // Redis connection
    pub redis: RedisService,
    pub current_connection_name: Option<String>,
    pub server_info: Option<ServerInfo>,

    // Connections management
    pub connection_list: ConnectionList,
    pub connection_form: ConnectionForm,

    // Database management
    pub db_list: Vec<DbInfo>,
    pub current_db_index: u32,
    pub is_db_selector_open: bool,
    pub db_selector_state: ListState,

    // Keys management
    pub keys: Vec<String>,
    pub key_list_state: ListState,
    pub key_search_filter: String,
    pub is_searching_keys: bool,
    pub selected_keys: std::collections::HashSet<String>,
    pub is_delete_confirmation_open: bool,

    // Key value viewer
    pub current_value: String,
    pub is_json_content: bool,
    pub scroll_offset: u16,
    pub json_editor: TextArea<'a>,
    pub cached_highlighted_json: Option<Vec<ratatui::text::Line<'static>>>,

    // Command input
    pub command_input: String,
    pub command_output: String, // Current command output

    // pending execution
    pub pending_connection: bool,
    pub pending_dashboard_data: bool,

    // Loading states
    pub is_connecting: bool,
    pub is_loading_keys: bool,
    pub is_loading_server_info: bool,
    pub is_loading_value: bool,

    // Loading animation
    pub loading_frame: usize,
    pub connection_delay_frames: usize,

    // Scroll throttling
    pub last_scroll_time: std::time::Instant,

    // Error handling
    pub error_message: Option<String>,
}

impl<'a> App<'a> {
    pub fn new() -> Self {
        let mut key_list_state = ListState::default();
        key_list_state.select(Some(0));

        let connection_list = ConnectionList::new(load_connections());

        let mut db_selector_state = ListState::default();
        db_selector_state.select(Some(0));

        Self {
            current_screen: CurrentScreen::ConnectionList,
            redis: RedisService::new(),
            current_connection_name: None,
            server_info: None,
            connection_list,
            connection_form: ConnectionForm::default(),
            db_list: Vec::new(),
            current_db_index: 0,
            is_db_selector_open: false,
            db_selector_state,
            keys: Vec::new(),
            key_list_state,
            key_search_filter: String::new(),
            is_searching_keys: false,
            selected_keys: std::collections::HashSet::new(),
            is_delete_confirmation_open: false,
            current_value: String::new(),
            is_json_content: false,
            scroll_offset: 0,
            json_editor: TextArea::default(),
            cached_highlighted_json: None,
            pending_connection: false,
            pending_dashboard_data: false,
            is_connecting: false,
            is_loading_keys: false,
            is_loading_server_info: false,
            is_loading_value: false,
            loading_frame: 0,
            connection_delay_frames: 0,
            last_scroll_time: std::time::Instant::now(),
            error_message: None,
            command_input: String::new(),
            command_output: String::new(),
        }
    }

    pub fn start_connection(&mut self) {
        self.pending_connection = true;
        self.is_connecting = true;
        self.error_message = None;
        self.connection_delay_frames = 8;

        // Clear previous connection state
        self.current_value.clear();
        self.is_json_content = false;
        self.scroll_offset = 0;
    }

    pub fn should_execute_connection(&mut self) -> bool {
        if self.pending_connection && self.connection_delay_frames > 0 {
            self.connection_delay_frames -= 1;
            false
        } else {
            self.pending_connection
        }
    }

    pub fn should_execute_dashboard_data(&mut self) -> bool {
        self.pending_dashboard_data && self.current_screen == CurrentScreen::Dashboard
    }

    pub async fn load_dashboard_data(&mut self) -> Result<()> {
        self.pending_dashboard_data = false;

        let _ = self.fetch_keys("*").await;

        let _ = self.load_server_info().await;

        Ok(())
    }

    pub async fn connect_to_selected(&mut self) -> Result<()> {
        self.pending_connection = false;

        if let Some(conn) = self.connection_list.selected_connection() {
            let mut url_str = if conn.use_tls {
                "rediss://".to_string()
            } else {
                "redis://".to_string()
            };

            if let Some(pass) = &conn.password {
                if !pass.is_empty() {
                    if let Some(user) = &conn.username {
                        if !user.is_empty() {
                            url_str.push_str(user);
                        }
                    }
                    url_str.push(':');
                    url_str.push_str(pass);
                    url_str.push('@');
                }
            }

            url_str.push_str(&conn.host);
            url_str.push(':');
            url_str.push_str(&conn.port.to_string());

            match self.redis.connect(&url_str).await {
                Ok(_) => {
                    self.is_connecting = false;
                    self.loading_frame = 0;
                    self.current_db_index = 0; // Reset to db0 on new connection

                    // Switch to Dashboard after successful connection
                    self.current_screen = CurrentScreen::Dashboard;
                    self.current_connection_name = Some(conn.name.clone());

                    // Need to load dashboard data - set loading indicators
                    self.pending_dashboard_data = true;
                    self.is_loading_keys = true;
                    self.is_loading_server_info = true;
                }
                Err(e) => {
                    self.is_connecting = false;
                    self.loading_frame = 0;
                    self.error_message = Some(format!("Connection failed: {}", e));
                    // Go back to connection list on error
                    self.current_screen = CurrentScreen::ConnectionList;
                    self.current_connection_name = None;
                }
            }
        }
        Ok(())
    }

    pub async fn fetch_keys(&mut self, pattern: &str) -> Result<()> {
        match self.redis.get_keys(pattern, self.current_db_index).await {
            Ok(keys) => {
                self.keys = keys;
                if !self.keys.is_empty() {
                    self.key_list_state.select(Some(0));
                } else {
                    self.key_list_state.select(None);
                }
                self.selected_keys.clear(); // Clear selection when refreshing keys
                self.error_message = None;
            }
            Err(e) => {
                self.error_message = Some(e.to_string());
            }
        }
        self.is_loading_keys = false;
        Ok(())
    }

    pub async fn fetch_value(&mut self, switch_screen: bool) -> Result<()> {
        let Some(selected) = self.key_list_state.selected() else {
            return Ok(());
        };
        let Some(key) = self.keys.get(selected) else {
            return Ok(());
        };

        match self.redis.get_value(key, self.current_db_index).await {
            Ok(val) => {
                // Quick check: JSON must start with {, [, or " and be non-empty
                let looks_like_json = val
                    .trim_start()
                    .starts_with(|c| c == '{' || c == '[' || c == '"');

                // Try formatting JSON only if it looks like JSON
                let (content, is_json) = if looks_like_json {
                    serde_json::from_str::<serde_json::Value>(&val)
                        .ok()
                        .and_then(|json| serde_json::to_string_pretty(&json).ok())
                        .map(|pretty| (pretty, true))
                        .unwrap_or_else(|| (val, false))
                } else {
                    (val, false)
                };

                self.current_value = content.clone();
                self.json_editor = TextArea::from(content.lines());
                self.is_json_content = is_json;
                self.cached_highlighted_json = None; // Clear cache when loading new value
                self.command_output.clear();
                self.error_message = None;
                self.scroll_offset = 0;

                if switch_screen {
                    self.current_screen = CurrentScreen::KeyContent;
                }
            }
            Err(e) => {
                self.error_message = Some(e.to_string());
            }
        }
        self.is_loading_value = false;
        Ok(())
    }

    pub async fn save_current_value(&mut self) -> Result<()> {
        let Some(selected) = self.key_list_state.selected() else {
            return Ok(());
        };
        let Some(key) = self.keys.get(selected) else {
            return Ok(());
        };

        match self.redis.set_value(key, &self.current_value, self.current_db_index).await {
            Ok(_) => {
                self.error_message = None;
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to save: {}", e));
            }
        }
        Ok(())
    }



    pub fn scroll_up(&mut self) {
        // Throttle: only allow scroll every 100ms to handle trackpad inertia
        let now = std::time::Instant::now();
        if now.duration_since(self.last_scroll_time).as_millis() < 16  {
            return;
        }
        self.last_scroll_time = now;
        self.scroll_offset = self.scroll_offset.saturating_sub(2);
    }

    pub fn scroll_down(&mut self) {
        // Throttle: only allow scroll every 100ms to handle trackpad inertia
        let now = std::time::Instant::now();
        if now.duration_since(self.last_scroll_time).as_millis() < 16  {
            return;
        }
        self.last_scroll_time = now;
        self.scroll_offset = self.scroll_offset.saturating_add(2);
    }



    pub fn tick_loading(&mut self) {
        // Use a large cycle to support different spinner lengths
        // The actual spinner in ui module will use modulo based on its own length
        self.loading_frame = (self.loading_frame + 1) % 1000;
    }

    pub fn next_connection(&mut self) {
        self.connection_list.next();
    }

    pub fn previous_connection(&mut self) {
        self.connection_list.previous();
    }

    pub fn delete_selected_connection(&mut self) -> Result<()> {
        if self.connection_list.delete_selected().is_some() {
            save_connections(self.connection_list.connections())?;
            self.error_message = None;
        }
        Ok(())
    }

    pub fn load_connection_for_edit(&mut self) {
        if let Some(conn) = self.connection_list.selected_connection() {
            self.connection_form = ConnectionForm::from_connection_config(conn);
            self.current_screen = CurrentScreen::NewConnectionForm;
            self.error_message = None;
        }
    }

    pub fn next_key(&mut self) {
        let now = std::time::Instant::now();
        if now.duration_since(self.last_scroll_time).as_millis() < 16  {
            return;
        }
        self.last_scroll_time = now;

        if self.keys.is_empty() {
            return;
        }
        let i = match self.key_list_state.selected() {
            Some(i) => {
                if i >= self.keys.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.key_list_state.select(Some(i));
    }

    pub fn previous_key(&mut self) {
        let now = std::time::Instant::now();
        if now.duration_since(self.last_scroll_time).as_millis() < 16  {
            return;
        }
        self.last_scroll_time = now;

        if self.keys.is_empty() {
            return;
        }
        let i = match self.key_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.keys.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.key_list_state.select(Some(i));
    }

    pub fn next_form_field(&mut self) {
        self.connection_form.next_field();
    }

    pub fn previous_form_field(&mut self) {
        self.connection_form.previous_field();
    }

    pub fn validate_connection_form(&mut self) -> bool {
        self.connection_form.validate()
    }

    pub fn get_filtered_keys(&self) -> Vec<String> {
        if self.key_search_filter.is_empty() {
            self.keys.clone()
        } else {
            self.keys
                .iter()
                .filter(|key| {
                    key.to_lowercase()
                        .contains(&self.key_search_filter.to_lowercase())
                })
                .cloned()
                .collect()
        }
    }

    pub async fn load_server_info(&mut self) -> Result<()> {
        match self.redis.get_server_info().await {
            Ok((info, db_list)) => {
                self.server_info = Some(info);
                self.db_list = db_list;
                // Update selector state to select current db
                if let Some(idx) = self
                    .db_list
                    .iter()
                    .position(|db| db.index == self.current_db_index)
                {
                    self.db_selector_state.select(Some(idx));
                } else {
                    self.db_selector_state.select(Some(0));
                }
                self.is_loading_server_info = false;
                Ok(())
            }
            Err(e) => {
                self.is_loading_server_info = false;
                self.error_message = Some(format!("Failed to get server info: {}", e));
                Err(e)
            }
        }
    }

    pub async fn switch_db(&mut self, db_index: u32) -> Result<()> {
        match self.redis.select_db(db_index).await {
            Ok(_) => {
                self.current_db_index = db_index;
                let _ = self.fetch_keys("*").await;
                Ok(())
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to switch database: {}", e));
                Err(e)
            }
        }
    }

    pub fn toggle_db_selector(&mut self) {
        self.is_db_selector_open = !self.is_db_selector_open;
    }

    pub fn next_db(&mut self) {
        if self.db_list.is_empty() {
            return;
        }
        let i = match self.db_selector_state.selected() {
            Some(i) => {
                if i >= self.db_list.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.db_selector_state.select(Some(i));
    }

    pub fn previous_db(&mut self) {
        if self.db_list.is_empty() {
            return;
        }
        let i = match self.db_selector_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.db_list.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.db_selector_state.select(Some(i));
    }

    pub async fn quick_import_from_clipboard(&mut self) -> Result<()> {
        // Read clipboard
        let clipboard_text = match arboard::Clipboard::new() {
            Ok(mut clipboard) => match clipboard.get_text() {
                Ok(text) => text,
                Err(e) => {
                    self.error_message = Some(format!("Failed to read clipboard: {}", e));
                    return Ok(());
                }
            },
            Err(e) => {
                self.error_message = Some(format!("Failed to access clipboard: {}", e));
                return Ok(());
            }
        };

        let clipboard_text = clipboard_text.trim();
        if clipboard_text.is_empty() {
            self.error_message = Some("Clipboard is empty".to_string());
            return Ok(());
        }

        // Parse connection string
        let mut form = match ConnectionForm::from_connection_string(clipboard_text) {
            Ok(form) => form,
            Err(e) => {
                self.error_message = Some(e);
                return Ok(());
            }
        };

        // Generate name if empty
        if form.name.is_empty() {
            let host_exists = self.connection_list.find_by_name(&form.host);
            form.name = if host_exists {
                format!("{}-{}", form.host, chrono::Local::now().format("%H%M%S"))
            } else {
                form.host.clone()
            };
        }

        // Parse db_aliases
        let db_aliases = if form.db_aliases.trim().is_empty() {
            std::collections::HashMap::new()
        } else {
            serde_json::from_str(&form.db_aliases).unwrap_or_default()
        };

        // Create connection config
        let new_conn = form.to_connection_config(db_aliases);

        // Save to config
        self.connection_list.push(new_conn.clone());
        if let Err(e) = save_connections(self.connection_list.connections()) {
            self.connection_list.delete_selected(); // Rollback
            self.error_message = Some(format!(
                "Failed to save connection '{}': {}\nConnection was not saved.",
                new_conn.name, e
            ));
            return Ok(());
        }

        // Clear error and start connection
        self.error_message = None;
        self.start_connection();

        Ok(())
    }

    pub async fn disconnect_and_return_to_list(&mut self) {
        // Disconnect from Redis
        self.redis.disconnect().await;

        // Reset to ConnectionList screen
        self.current_screen = CurrentScreen::ConnectionList;

        // Clear all dashboard data
        self.keys.clear();
        self.current_value.clear();
        self.is_json_content = false;
        self.scroll_offset = 0;

        // Reset header information
        self.current_connection_name = None;
        self.server_info = None;

        // Reset search keys input
        self.key_search_filter.clear();
        self.is_searching_keys = false;

        // Reset selection
        self.selected_keys.clear();
        self.is_delete_confirmation_open = false;

        // Reset database info
        self.db_list.clear();
        self.current_db_index = 0;
        self.is_db_selector_open = false;
    }

    pub fn save_connection_form(&mut self) -> Result<()> {
        if !self.validate_connection_form() {
            return Ok(());
        }

        // Parse db_aliases JSON string
        let db_aliases = if self.connection_form.db_aliases.trim().is_empty() {
            std::collections::HashMap::new()
        } else {
            match serde_json::from_str(&self.connection_form.db_aliases) {
                Ok(aliases) => aliases,
                Err(_) => {
                    self.connection_form.validation_error =
                        Some("Invalid DB Aliases JSON format (e.g., {\"0\": \"0\"})".to_string());
                    return Ok(());
                }
            }
        };

        let new_conn = self.connection_form.to_connection_config(db_aliases);

        // Check if it's edit mode or new mode
        if self.connection_form.editing_connection_id.is_some() {
            if let Err(e) = self.connection_list.update_selected(new_conn) {
                self.error_message = Some(format!("Failed to update: {}", e));
                return Ok(());
            }
        } else {
            self.connection_list.push(new_conn);
        }

        if let Err(e) = save_connections(self.connection_list.connections()) {
            self.error_message = Some(format!("Failed to save: {}", e));
        } else {
            self.connection_form = Default::default();
            self.error_message = None;
            self.current_screen = CurrentScreen::ConnectionList;
        }

        Ok(())
    }

    pub fn toggle_key_selection(&mut self) {
        if let Some(selected_idx) = self.key_list_state.selected() {
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

    pub fn select_all_keys(&mut self) {
        let filtered_keys = self.get_filtered_keys();
        for key in filtered_keys {
            self.selected_keys.insert(key);
        }
    }

    pub fn clear_key_selection(&mut self) {
        self.selected_keys.clear();
    }

    pub fn open_delete_confirmation(&mut self) {
        if !self.selected_keys.is_empty() {
            self.is_delete_confirmation_open = true;
        }
    }

    pub fn close_delete_confirmation(&mut self) {
        self.is_delete_confirmation_open = false;
    }

    pub async fn delete_selected_keys(&mut self) -> Result<()> {
        if self.selected_keys.is_empty() {
            return Ok(());
        }

        let keys_to_delete: Vec<String> = self.selected_keys.iter().cloned().collect();
        let mut deleted_count = 0;
        let mut errors = Vec::new();

        for key in &keys_to_delete {
            match self.redis.delete_key(key, self.current_db_index).await {
                Ok(_) => {
                    deleted_count += 1;
                }
                Err(e) => {
                    errors.push(format!("{}: {}", key, e));
                }
            }
        }

        // Update UI
        if deleted_count > 0 {
            // Remove deleted keys from the list
            self.keys.retain(|k| !keys_to_delete.contains(k));
            self.selected_keys.clear();

            // Adjust selection
            if !self.keys.is_empty() {
                let current_idx = self.key_list_state.selected().unwrap_or(0);
                let new_idx = current_idx.min(self.keys.len() - 1);
                self.key_list_state.select(Some(new_idx));
            } else {
                self.key_list_state.select(None);
            }

            // Update server info to reflect new key count
            let _ = self.load_server_info().await;
        }

        // Set error or success message
        if errors.is_empty() {
            self.error_message = None;
        } else if deleted_count > 0 {
            self.error_message = Some(format!(
                "Deleted {} keys, but {} failed: {}",
                deleted_count,
                errors.len(),
                errors.join(", ")
            ));
        } else {
            self.error_message = Some(format!("Failed to delete keys: {}", errors.join(", ")));
        }

        self.is_delete_confirmation_open = false;
        Ok(())
    }

    pub async fn execute_command(&mut self) -> Result<()> {
        if self.command_input.trim().is_empty() {
            return Ok(());
        }

        let command = self.command_input.trim().to_string();
        let parts: Vec<&str> = command.split_whitespace().collect();

        if parts.is_empty() {
            self.command_input.clear();
            return Ok(());
        }

        let mut output = match self.redis.execute_command(&parts, self.current_db_index).await {
            Ok(output) => {
                self.error_message = None;
                output
            }
            Err(e) => {
                let error_str = e.to_string();
                // Check for connection errors and auto-reconnect
                if error_str.contains("broken pipe")
                    || error_str.contains("Connection refused")
                    || error_str.contains("Connection reset")
                {
                    self.error_message = Some("Connection lost, reconnecting...".to_string());
                    self.start_connection();
                    return Ok(());
                }
                format!("(error) {}", error_str)
            }
        };

        // Store output for display (with size limit)
        const MAX_OUTPUT_SIZE: usize = 500 * 1024; // 500KB
        if output.len() > MAX_OUTPUT_SIZE {
            let original_len = output.len();
            output.truncate(MAX_OUTPUT_SIZE);
            output.push_str(&format!(
                "\n\n[Output truncated - too large ({} bytes). Use smaller queries or pagination.]",
                original_len
            ));
        }

        self.command_output = output.clone();
        
        // Check if output is JSON and format it
        let trimmed = output.trim();
        if (trimmed.starts_with('{') && trimmed.ends_with('}')) || 
           (trimmed.starts_with('[') && trimmed.ends_with(']')) {
            // Try to parse and pretty-print JSON
            match serde_json::from_str::<serde_json::Value>(trimmed) {
                Ok(json_value) => {
                    if let Ok(formatted) = serde_json::to_string_pretty(&json_value) {
                        self.current_value = formatted;
                        self.is_json_content = true;
                    } else {
                        self.current_value = output;
                        self.is_json_content = false;
                    }
                }
                Err(_) => {
                    // Not valid JSON, treat as plain text
                    self.current_value = output;
                    self.is_json_content = false;
                }
            }
        } else {
            // Not JSON format
            self.current_value = output;
            self.is_json_content = false;
        }
        
        self.cached_highlighted_json = None; // Clear cache for new content
        
        self.command_input.clear();

        // Reset scroll to top for new output
        self.scroll_offset = 0;

        Ok(())
    }

    pub fn toggle_command_mode(&mut self) {
        if self.current_screen == CurrentScreen::CommandMode {
            // Exit command mode - return to Dashboard
            self.current_screen = CurrentScreen::Dashboard;
            // Clear all CLI state when exiting command mode
            self.command_input.clear();
            self.command_output.clear();
            self.current_value.clear();
            self.is_json_content = false;
            self.scroll_offset = 0;
            self.cached_highlighted_json = None;
        } else {
            // Enter command mode
            self.current_screen = CurrentScreen::CommandMode;
        }
    }


}
