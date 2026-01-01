use std::collections::HashMap;

use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use unicode_width::UnicodeWidthStr;

use crate::models::ConnectionConfig;
use crate::theme::get_colors;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FormField {
    Name,
    Host,
    Port,
    Username,
    Password,
    UseTls,
    AllowInsecureTls,
    Sni,
    ClusterNodes,
    DbAliases,
}

#[derive(Debug, Clone)]
pub enum Message {
    NextField,
    PreviousField,
    ToggleClusterMode,
    UpdateField(char),
    Backspace,
    ToggleCheckbox,
    Save,
    Cancel,
    OpenNew,
    OpenEdit(Box<ConnectionConfig>),
}

#[derive(Debug, Clone)]
pub enum UpdateResult {
    None,
    Saved(Box<ConnectionConfig>),
    Cancelled,
    ValidationError,
}

#[derive(Debug, Clone)]
pub struct ConnectionForm {
    pub name: String,
    pub host: String,
    pub port: String,
    pub password: Option<String>,
    pub username: Option<String>,
    pub use_tls: bool,
    pub allow_insecure_tls: bool,
    pub sni: String,
    pub db_aliases: String,
    pub is_cluster: bool,
    pub cluster_nodes: String,
    pub editing_field: FormField,
    pub validation_error: Option<String>,
    pub editing_connection_id: Option<String>,
    pub is_open: bool,
}

impl ConnectionForm {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            host: "127.0.0.1".to_string(),
            port: "6379".to_string(),
            password: None,
            username: None,
            use_tls: false,
            allow_insecure_tls: false,
            sni: String::new(),
            is_cluster: false,
            cluster_nodes: String::new(),
            db_aliases: "{\"0\":\"0\"}".to_string(),
            editing_field: FormField::Name,
            validation_error: None,
            editing_connection_id: None,
            is_open: false,
        }
    }

    fn is_checkbox_field(&self) -> bool {
        matches!(
            self.editing_field,
            FormField::UseTls | FormField::AllowInsecureTls
        )
    }

    pub fn handle_key_events(&self, key: KeyEvent) -> Option<Message> {
        if !self.is_open {
            return None;
        }
        match (key.code, key.modifiers) {
            (KeyCode::Tab, _) => Some(Message::ToggleClusterMode),
            (KeyCode::Up, _) => Some(Message::PreviousField),
            (KeyCode::Down, _) => Some(Message::NextField),
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                Some(Message::UpdateField(c))
            }
            (KeyCode::Backspace, _) => Some(Message::Backspace),
            (KeyCode::Enter, _) | (KeyCode::Char(' '), _) if self.is_checkbox_field() => {
                Some(Message::ToggleCheckbox)
            }
            (KeyCode::Enter, _) => Some(Message::NextField),
            (KeyCode::Char(' '), _) => Some(Message::UpdateField(' ')),
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => Some(Message::Save),
            (KeyCode::Esc, _) => Some(Message::Cancel),
            _ => None,
        }
    }

    pub fn update(&mut self, msg: Message) -> UpdateResult {
        match msg {
            Message::NextField => {
                self.next_field();
                UpdateResult::None
            }
            Message::PreviousField => {
                self.previous_field();
                UpdateResult::None
            }
            Message::ToggleClusterMode => {
                self.is_cluster = !self.is_cluster;
                UpdateResult::None
            }
            Message::UpdateField(c) => {
                self.update_current_field(c);
                UpdateResult::None
            }
            Message::Backspace => {
                self.backspace_current_field();
                UpdateResult::None
            }
            Message::ToggleCheckbox => {
                self.toggle_current_checkbox();
                UpdateResult::None
            }
            Message::Save => {
                if self.validate() {
                    let db_aliases = serde_json::from_str(&self.db_aliases)
                        .unwrap_or_else(|_| HashMap::from([(0, "0".to_string())]));
                    let config = self.to_connection_config(db_aliases);
                    self.is_open = false;
                    UpdateResult::Saved(Box::new(config))
                } else {
                    UpdateResult::ValidationError
                }
            }
            Message::Cancel => {
                self.is_open = false;
                UpdateResult::Cancelled
            }
            Message::OpenNew => {
                self.open_new();
                UpdateResult::None
            }
            Message::OpenEdit(config) => {
                self.open_edit(&config);
                UpdateResult::None
            }
        }
    }

    pub fn view(&mut self, frame: &mut Frame, area: Rect) -> Option<(u16, u16)> {
        let colors = get_colors();

        let title = if self.editing_connection_id.is_some() {
            "Edit Redis Connection"
        } else {
            "New Redis Connection"
        };

        let block = Block::default()
            .title(Line::from(vec![Span::styled(
                title,
                Style::default()
                    .fg(colors.text_primary)
                    .add_modifier(Modifier::BOLD),
            )]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(colors.border_active))
            .style(Style::default().bg(colors.bg_dialog));

        frame.render_widget(block, area);

        let inner = area.inner(Margin::new(2, 1));

        self.render_mode_tabs(frame, inner);

        // Form area below tabs
        let form_area = Rect {
            x: inner.x,
            y: inner.y + 2,
            width: inner.width,
            height: inner.height - 2,
        };

        self.render_form_fields(frame, form_area)
    }

    fn open_new(&mut self) {
        *self = Self::new();
        self.is_open = true;
    }

    fn open_edit(&mut self, config: &ConnectionConfig) {
        *self = Self::from_connection_config(config);
        self.is_open = true;
    }

    fn next_field(&mut self) {
        let fields = self.get_field_order();
        if let Some(pos) = fields.iter().position(|&f| f == self.editing_field) {
            self.editing_field = fields[(pos + 1) % fields.len()];
        }
    }

    fn previous_field(&mut self) {
        let fields = self.get_field_order();
        if let Some(pos) = fields.iter().position(|&f| f == self.editing_field) {
            self.editing_field = fields[(pos + fields.len() - 1) % fields.len()];
        }
    }

    fn get_field_order(&self) -> Vec<FormField> {
        let mut fields = vec![FormField::Name];
        if self.is_cluster {
            fields.push(FormField::ClusterNodes);
        } else {
            fields.extend([FormField::Host, FormField::Port]);
        }
        fields.extend([
            FormField::Username,
            FormField::Password,
            FormField::UseTls,
            FormField::AllowInsecureTls,
            FormField::Sni,
            FormField::DbAliases,
        ]);
        fields
    }

    fn validate(&mut self) -> bool {
        self.validation_error = None;

        if self.name.trim().is_empty() {
            self.validation_error = Some("Connection name cannot be empty".to_string());
            return false;
        }

        if !self.is_cluster {
            if self.host.trim().is_empty() {
                self.validation_error = Some("Host address cannot be empty".to_string());
                return false;
            }

            if self.port.trim().is_empty() {
                self.validation_error = Some("Port number cannot be empty".to_string());
                return false;
            }

            if self.port.parse::<u16>().is_err() {
                self.validation_error = Some("Port must be a valid number (1-65535)".to_string());
                return false;
            }
        } else if self.cluster_nodes.trim().is_empty() {
            self.validation_error = Some("Cluster nodes cannot be empty".to_string());
            return false;
        }

        true
    }

    fn parse_db_aliases(&self) -> HashMap<u32, String> {
        if self.db_aliases.trim().is_empty() {
            HashMap::new()
        } else {
            serde_json::from_str(&self.db_aliases).unwrap_or_default()
        }
    }

    fn generate_name_if_empty(&mut self, existing_names: &[String]) {
        if !self.name.is_empty() {
            return;
        }

        let (base_name, suffix) = if self.is_cluster {
            let first_node = self
                .cluster_nodes
                .split(',')
                .next()
                .unwrap_or("cluster")
                .trim();
            let base = first_node.split(':').next().unwrap_or(first_node);
            (base.to_string(), "-cluster".to_string())
        } else {
            (self.host.clone(), String::new())
        };

        let name_exists = existing_names.iter().any(|name| {
            name == &base_name || name.starts_with(&format!("{}{}", base_name, suffix))
        });
        self.name = if name_exists {
            format!(
                "{}{}-{}",
                base_name,
                suffix,
                chrono::Local::now().format("%H%M%S")
            )
        } else {
            format!("{}{}", base_name, suffix)
        };
    }

    fn to_connection_config(&self, db_aliases: HashMap<u32, String>) -> ConnectionConfig {
        ConnectionConfig {
            id: self
                .editing_connection_id
                .clone()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            name: self.name.clone(),
            host: self.host.clone(),
            port: self.port.parse().unwrap_or(6379),
            password: self.password.clone().filter(|s| !s.is_empty()),
            username: self.username.clone().filter(|s| !s.is_empty()),
            use_tls: self.use_tls,
            allow_insecure_tls: self.allow_insecure_tls,
            sni: (!self.sni.is_empty()).then(|| self.sni.clone()),
            is_cluster: self.is_cluster,
            cluster_nodes: if self.is_cluster {
                self.cluster_nodes
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .collect()
            } else {
                Vec::new()
            },
            db_aliases,
        }
    }

    fn from_connection_config(config: &ConnectionConfig) -> Self {
        Self {
            name: config.name.clone(),
            host: config.host.clone(),
            port: config.port.to_string(),
            password: config.password.clone(),
            username: config.username.clone(),
            use_tls: config.use_tls,
            allow_insecure_tls: config.allow_insecure_tls,
            sni: config.sni.clone().unwrap_or_default(),
            is_cluster: config.is_cluster,
            cluster_nodes: config.cluster_nodes.join(", "),
            db_aliases: serde_json::to_string(&config.db_aliases)
                .unwrap_or_else(|_| "{\"0\":\"0\"}".to_string()),
            editing_field: FormField::Name,
            validation_error: None,
            editing_connection_id: Some(config.id.clone()),
            is_open: false,
        }
    }

    fn from_connection_string(input: &str) -> Result<Self, String> {
        let mut form = Self::new();
        Self::parse_connection_string(input, &mut form);

        if form.is_cluster {
            if form.cluster_nodes.trim().is_empty() {
                return Err("Failed to parse cluster connection: no nodes found".to_string());
            }

            let nodes: Vec<&str> = form.cluster_nodes.split(',').collect();
            for node in &nodes {
                let node = node.trim();
                if !node.contains(':') {
                    return Err(format!(
                        "Failed to parse cluster node '{}': must be in format host:port",
                        node
                    ));
                }
            }
        } else {
            if form.host.trim().is_empty() {
                return Err(format!(
                    "Failed to parse connection string: could not extract host\nInput: {}",
                    if input.len() > 60 {
                        format!("{}...", &input[..60])
                    } else {
                        input.to_string()
                    }
                ));
            }

            let host = form.host.trim();
            if !Self::is_valid_host(host) {
                return Err(format!(
                    "Failed to parse connection string: invalid host '{}'\nHost must be a valid hostname or IP address",
                    host
                ));
            }

            if form.port.trim().is_empty() {
                form.port = "6379".to_string();
            }

            if form.port.parse::<u16>().is_err() {
                return Err(format!(
                    "Failed to parse connection string: invalid port '{}'\nHost: {}",
                    form.port, form.host
                ));
            }

            if form.use_tls && form.sni.trim().is_empty() {
                form.sni = form.host.clone();
            }
        }

        Ok(form)
    }

    pub fn import_from_string(
        input: &str,
        existing_names: &[String],
    ) -> Result<ConnectionConfig, String> {
        let mut form = Self::from_connection_string(input)?;
        form.generate_name_if_empty(existing_names);
        let db_aliases = form.parse_db_aliases();
        Ok(form.to_connection_config(db_aliases))
    }

    fn parse_connection_string(input: &str, form: &mut Self) {
        let mut url_to_parse = String::new();
        let mut override_host = None;
        let mut override_port = None;
        let mut override_user = None;
        let mut override_pass = None;
        let mut override_tls = false;
        let mut override_sni = None;
        let mut is_cluster_mode = false;

        if input.starts_with("redis-cli") {
            let parts: Vec<&str> = input.split_whitespace().collect();
            let mut i = 1;
            while i < parts.len() {
                let take_next = |i: &mut usize| -> Option<String> {
                    if *i + 1 < parts.len() {
                        *i += 1;
                        Some(parts[*i].to_string())
                    } else {
                        None
                    }
                };
                match parts[i] {
                    "-c" | "--cluster" => is_cluster_mode = true,
                    "-u" => {
                        if let Some(v) = take_next(&mut i) {
                            url_to_parse = v;
                        }
                    }
                    "-h" => {
                        if let Some(v) = take_next(&mut i) {
                            override_host = Some(v);
                        }
                    }
                    "-p" => {
                        if let Some(v) = take_next(&mut i) {
                            override_port = Some(v);
                        }
                    }
                    "-a" => {
                        if let Some(v) = take_next(&mut i) {
                            override_pass = Some(v);
                        }
                    }
                    "--user" => {
                        if let Some(v) = take_next(&mut i) {
                            override_user = Some(v);
                        }
                    }
                    "--tls" => override_tls = true,
                    "--sni" => {
                        if let Some(v) = take_next(&mut i) {
                            override_sni = Some(v);
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
        } else {
            url_to_parse = input.to_string();
        }

        if !url_to_parse.is_empty() {
            let (scheme, rest) = if let Some(stripped) = url_to_parse.strip_prefix("rediss://") {
                form.use_tls = true;
                ("rediss://", stripped)
            } else if let Some(stripped) = url_to_parse.strip_prefix("redis://") {
                ("redis://", stripped)
            } else {
                ("", url_to_parse.as_str())
            };

            let after_auth = if let Some(idx) = rest.rfind('@') {
                &rest[idx + 1..]
            } else {
                rest
            };

            if after_auth.contains(',') {
                is_cluster_mode = true;
                Self::parse_cluster_nodes(rest, form, scheme);
            } else {
                Self::parse_authority(rest, form);
            }
        }

        // Apply overrides
        if let Some(h) = override_host {
            if is_cluster_mode {
                form.cluster_nodes =
                    format!("{}:{}", h, override_port.as_deref().unwrap_or("6379"));
            } else {
                form.host = h;
            }
        }
        if let Some(p) = override_port.filter(|_| !is_cluster_mode) {
            form.port = p;
        }
        if let Some(u) = override_user {
            form.username = Some(u);
        }
        if let Some(p) = override_pass {
            form.password = Some(p);
        }
        if override_tls {
            form.use_tls = true;
        }
        if let Some(s) = override_sni {
            form.sni = s;
        }

        form.is_cluster = is_cluster_mode;
    }

    fn parse_auth(auth_str: &str, form: &mut Self) {
        if let Some(idx) = auth_str.find(':') {
            form.username = Some(auth_str[..idx].to_string());
            form.password = Some(auth_str[idx + 1..].to_string());
        } else {
            form.password = Some(auth_str.to_string());
        }
    }

    fn parse_cluster_nodes(input: &str, form: &mut Self, _scheme: &str) {
        let nodes_part = if let Some(idx) = input.rfind('@') {
            Self::parse_auth(&input[..idx], form);
            &input[idx + 1..]
        } else {
            input
        };

        form.cluster_nodes = nodes_part
            .split(',')
            .map(|node| node.trim().split('/').next().unwrap_or(node.trim()))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
    }

    fn parse_authority(authority: &str, form: &mut Self) {
        let (user_pass, host_port) = if let Some(idx) = authority.rfind('@') {
            (Some(&authority[..idx]), &authority[idx + 1..])
        } else {
            (None, authority)
        };

        if let Some(up) = user_pass {
            Self::parse_auth(up, form);
        }

        let host_port_no_db = host_port.split('/').next().unwrap_or(host_port);
        if let Some(idx) = host_port_no_db.rfind(':') {
            form.host = host_port_no_db[..idx].to_string();
            form.port = host_port_no_db[idx + 1..].to_string();
        } else {
            form.host = host_port_no_db.to_string();
            form.port = "6379".to_string();
        }
    }

    fn is_valid_host(host: &str) -> bool {
        // Check IPv4
        if host.split('.').count() == 4 && host.split('.').all(|part| part.parse::<u8>().is_ok()) {
            return true;
        }

        // Basic validation
        if host.is_empty() || host.len() > 253 {
            return false;
        }

        // Check invalid characters
        if host.contains(|c: char| !c.is_ascii_alphanumeric() && c != '.' && c != '-' && c != '_') {
            return false;
        }

        // Check boundaries
        if host.starts_with('-')
            || host.ends_with('-')
            || host.starts_with('.')
            || host.ends_with('.')
        {
            return false;
        }

        // Validate labels
        host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
        })
    }

    fn get_field_mut(&mut self) -> Option<&mut String> {
        match self.editing_field {
            FormField::Name => Some(&mut self.name),
            FormField::Host => Some(&mut self.host),
            FormField::Port => Some(&mut self.port),
            FormField::Username => self.username.get_or_insert_with(String::new).into(),
            FormField::Password => self.password.get_or_insert_with(String::new).into(),
            FormField::Sni => Some(&mut self.sni),
            FormField::ClusterNodes => Some(&mut self.cluster_nodes),
            FormField::DbAliases => Some(&mut self.db_aliases),
            FormField::UseTls | FormField::AllowInsecureTls => None,
        }
    }

    fn update_current_field(&mut self, c: char) {
        if let Some(field) = self.get_field_mut() {
            field.push(c);
        }
    }

    fn backspace_current_field(&mut self) {
        if let Some(field) = self.get_field_mut() {
            field.pop();
        }
    }

    fn toggle_current_checkbox(&mut self) {
        match self.editing_field {
            FormField::UseTls => self.use_tls = !self.use_tls,
            FormField::AllowInsecureTls => self.allow_insecure_tls = !self.allow_insecure_tls,
            _ => {}
        }
    }

    fn render_mode_tabs(&self, frame: &mut Frame, area: Rect) {
        let colors = get_colors();
        let active_style = Style::default()
            .fg(colors.border_active)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
        let inactive_style = Style::default().fg(colors.text_secondary);

        let tabs = Line::from(vec![
            Span::styled(
                "[ Standalone ]",
                if !self.is_cluster {
                    active_style
                } else {
                    inactive_style
                },
            ),
            Span::raw("  "),
            Span::styled(
                "[ Cluster ]",
                if self.is_cluster {
                    active_style
                } else {
                    inactive_style
                },
            ),
            Span::raw("  "),
            Span::styled(
                "(Press Tab to switch)",
                Style::default()
                    .fg(colors.text_secondary)
                    .add_modifier(Modifier::DIM),
            ),
        ]);

        frame.render_widget(
            Paragraph::new(tabs),
            Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 1,
            },
        );
    }

    fn render_form_fields(&self, frame: &mut Frame, area: Rect) -> Option<(u16, u16)> {
        let mut constraints = vec![
            Constraint::Length(3), // Name
            Constraint::Length(3), // Host/Port or Cluster
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Username/Password
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // TLS options
            Constraint::Length(3), // SNI
            Constraint::Length(3), // DB Aliases
        ];
        if self.validation_error.is_some() {
            constraints.extend([Constraint::Length(1), Constraint::Length(3)]); // Spacer + Error
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        let mut cursor_pos = None;
        let mut idx = 0;

        // Connection Name
        cursor_pos = self
            .render_text_field(
                frame,
                chunks[idx],
                "Connection Name",
                &self.name,
                FormField::Name,
                true,
                None,
                "...",
            )
            .or(cursor_pos);
        idx += 1;

        // Host & Port OR Cluster Nodes
        cursor_pos = if self.is_cluster {
            self.render_text_field(
                frame,
                chunks[idx],
                "Cluster Nodes",
                &self.cluster_nodes,
                FormField::ClusterNodes,
                true,
                Some("e.g. host1:6379, host2:6379"),
                "...",
            )
            .or(cursor_pos)
        } else {
            self.render_host_port_fields(frame, chunks[idx])
                .or(cursor_pos)
        };
        idx += 2; // Skip spacer

        // Username & Password
        cursor_pos = self.render_auth_fields(frame, chunks[idx]).or(cursor_pos);
        idx += 2; // Skip spacer

        // TLS Options
        self.render_tls_fields(frame, chunks[idx]);
        idx += 1;

        // SNI
        cursor_pos = self
            .render_text_field(
                frame,
                chunks[idx],
                "SNI",
                &self.sni,
                FormField::Sni,
                false,
                Some("optional"),
                "...",
            )
            .or(cursor_pos);
        idx += 1;

        // DB Aliases
        cursor_pos = self
            .render_text_field(
                frame,
                chunks[idx],
                "DB Aliases",
                &self.db_aliases,
                FormField::DbAliases,
                false,
                Some("optional, JSON"),
                "...",
            )
            .or(cursor_pos);
        idx += 1;

        // Validation Error
        if let Some(ref error) = self.validation_error {
            idx += 1; // Skip spacer
            let colors = get_colors();
            let error_text = Paragraph::new(Line::from(vec![Span::styled(
                error,
                Style::default().fg(colors.error),
            )]))
            .alignment(ratatui::layout::Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(colors.border_default)),
            );
            frame.render_widget(error_text, chunks[idx]);
        }

        cursor_pos
    }

    fn render_host_port_fields(&self, frame: &mut Frame, area: Rect) -> Option<(u16, u16)> {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(area);

        let mut cursor_pos = None;

        // Host
        if let Some(pos) = self.render_text_field(
            frame,
            chunks[0],
            "Host",
            &self.host,
            FormField::Host,
            true,
            None,
            "127.0.0.1",
        ) {
            cursor_pos = Some(pos);
        }

        // Port
        if let Some(pos) = self.render_text_field(
            frame,
            chunks[1],
            "Port",
            &self.port,
            FormField::Port,
            true,
            None,
            "6379",
        ) {
            cursor_pos = Some(pos);
        }

        cursor_pos
    }

    fn render_auth_fields(&self, frame: &mut Frame, area: Rect) -> Option<(u16, u16)> {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let mut cursor_pos = None;

        // Username
        if let Some(pos) = self.render_text_field(
            frame,
            chunks[0],
            "Username",
            self.username.as_deref().unwrap_or(""),
            FormField::Username,
            false,
            Some("optional"),
            "...",
        ) {
            cursor_pos = Some(pos);
        }

        // Password
        if let Some(pos) = self.render_text_field(
            frame,
            chunks[1],
            "Password",
            self.password.as_deref().unwrap_or(""),
            FormField::Password,
            false,
            Some("optional"),
            "...",
        ) {
            cursor_pos = Some(pos);
        }

        cursor_pos
    }

    fn render_tls_fields(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Use TLS
        self.render_checkbox_field(frame, chunks[0], "Use TLS", self.use_tls, FormField::UseTls);

        // Allow Insecure TLS
        self.render_checkbox_field(
            frame,
            chunks[1],
            "Allow Insecure TLS",
            self.allow_insecure_tls,
            FormField::AllowInsecureTls,
        );
    }

    fn get_field_styles(&self, field: FormField) -> (ratatui::style::Color, ratatui::style::Color) {
        let colors = get_colors();
        let is_active = self.editing_field == field;
        (
            if is_active {
                colors.border_active
            } else {
                colors.border_default
            },
            if is_active {
                colors.border_active
            } else {
                colors.text_primary
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_text_field(
        &self,
        frame: &mut Frame,
        area: Rect,
        label: &str,
        value: &str,
        field: FormField,
        required: bool,
        hint: Option<&str>,
        placeholder: &str,
    ) -> Option<(u16, u16)> {
        let colors = get_colors();
        let (border_color, title_color) = self.get_field_styles(field);
        let is_active = self.editing_field == field;

        let mut title_spans = vec![Span::styled(
            label,
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        )];

        if required {
            title_spans.push(Span::styled(
                " *",
                Style::default()
                    .fg(colors.text_required)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        if let Some(hint_text) = hint {
            title_spans.push(Span::styled(
                format!(" ({})", hint_text),
                Style::default().fg(title_color),
            ));
        }

        let display_value = if value.is_empty() && !is_active {
            placeholder
        } else {
            value
        };

        let value_style = if value.is_empty() && !is_active {
            Style::default()
                .fg(colors.text_disabled)
                .add_modifier(Modifier::DIM)
        } else {
            Style::default().fg(colors.text_primary)
        };

        let widget = Paragraph::new(display_value).style(value_style).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .title(Line::from(title_spans)),
        );

        frame.render_widget(widget, area);

        if is_active {
            Some((area.x + 1 + value.width() as u16, area.y + 1))
        } else {
            None
        }
    }

    fn render_checkbox_field(
        &self,
        frame: &mut Frame,
        area: Rect,
        label: &str,
        checked: bool,
        field: FormField,
    ) {
        let colors = get_colors();
        let (border_color, title_color) = self.get_field_styles(field);

        let title_span = Line::from(vec![Span::styled(
            label,
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        )]);

        let check_display = if checked {
            Line::from(vec![
                Span::styled(
                    "✓ ",
                    Style::default()
                        .fg(colors.success)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("Enabled", Style::default().fg(colors.success)),
            ])
        } else {
            Line::from(vec![
                Span::styled("○ ", Style::default().fg(colors.text_secondary)),
                Span::styled("Disabled", Style::default().fg(colors.text_secondary)),
            ])
        };

        let widget = Paragraph::new(check_display).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .title(title_span),
        );

        frame.render_widget(widget, area);
    }
}
