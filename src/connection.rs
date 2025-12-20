use anyhow::Result;
use ratatui::widgets::ListState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub password: Option<String>,
    pub username: Option<String>,
    #[serde(default)]
    pub db_aliases: HashMap<u32, String>,
    #[serde(default)]
    pub use_tls: bool,
    #[serde(default)]
    pub allow_insecure_tls: bool,
    pub sni: Option<String>,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Localhost".to_string(),
            host: "127.0.0.1".to_string(),
            port: 6379,
            password: None,
            username: None,
            db_aliases: HashMap::new(),
            use_tls: false,
            allow_insecure_tls: false,
            sni: None,
        }
    }
}

pub struct ConnectionList {
    connections: Vec<ConnectionConfig>,
    state: ListState,
}

impl ConnectionList {
    pub fn new(connections: Vec<ConnectionConfig>) -> Self {
        let mut state = ListState::default();
        if !connections.is_empty() {
            state.select(Some(0));
        }
        Self { connections, state }
    }

    pub fn connections(&self) -> &[ConnectionConfig] {
        &self.connections
    }

    pub fn state(&mut self) -> &mut ListState {
        &mut self.state
    }

    pub fn selected_connection(&self) -> Option<&ConnectionConfig> {
        self.state.selected().and_then(|i| self.connections.get(i))
    }

    pub fn selected_connection_mut(&mut self) -> Option<&mut ConnectionConfig> {
        self.state
            .selected()
            .and_then(|i| self.connections.get_mut(i))
    }

    pub fn next(&mut self) {
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

    pub fn previous(&mut self) {
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

    pub fn push(&mut self, connection: ConnectionConfig) {
        self.connections.push(connection);
        // Auto-select the newly added connection
        let new_index = self.connections.len() - 1;
        self.state.select(Some(new_index));
    }

    pub fn delete_selected(&mut self) -> Option<ConnectionConfig> {
        if let Some(selected_idx) = self.state.selected() {
            if selected_idx < self.connections.len() {
                let removed = self.connections.remove(selected_idx);
                if self.connections.is_empty() {
                    self.state.select(None);
                } else {
                    let new_idx = if selected_idx > 0 {
                        selected_idx - 1
                    } else {
                        0
                    };
                    self.state.select(Some(new_idx));
                }
                return Some(removed);
            }
        }
        None
    }

    pub fn update_selected(&mut self, connection: ConnectionConfig) -> Result<()> {
        if let Some(conn) = self.selected_connection_mut() {
            *conn = connection;
            Ok(())
        } else {
            Err(anyhow::anyhow!("No connection selected"))
        }
    }

    pub fn find_by_name(&self, name: &str) -> bool {
        self.connections.iter().any(|c| c.name == name)
    }
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
    pub editing_field: FormField,
    pub validation_error: Option<String>,
    pub editing_connection_id: Option<String>,
}

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
    DbAliases,
    Submit,
}

impl Default for ConnectionForm {
    fn default() -> Self {
        Self {
            name: String::new(),
            host: "127.0.0.1".to_string(),
            port: "6379".to_string(),
            password: None,
            username: None,
            use_tls: false,
            allow_insecure_tls: false,
            sni: String::new(),
            db_aliases: "{\"0\":\"0\"}".to_string(),
            editing_field: FormField::Name,
            validation_error: None,
            editing_connection_id: None,
        }
    }
}

impl ConnectionForm {
    pub fn next_field(&mut self) {
        self.editing_field = match self.editing_field {
            FormField::Name => FormField::Host,
            FormField::Host => FormField::Port,
            FormField::Port => FormField::Username,
            FormField::Username => FormField::Password,
            FormField::Password => FormField::UseTls,
            FormField::UseTls => FormField::AllowInsecureTls,
            FormField::AllowInsecureTls => FormField::Sni,
            FormField::Sni => FormField::DbAliases,
            FormField::DbAliases => FormField::Submit,
            FormField::Submit => FormField::Name,
        };
    }

    pub fn previous_field(&mut self) {
        self.editing_field = match self.editing_field {
            FormField::Name => FormField::Submit,
            FormField::Host => FormField::Name,
            FormField::Port => FormField::Host,
            FormField::Username => FormField::Port,
            FormField::Password => FormField::Username,
            FormField::UseTls => FormField::Password,
            FormField::AllowInsecureTls => FormField::UseTls,
            FormField::Sni => FormField::AllowInsecureTls,
            FormField::DbAliases => FormField::Sni,
            FormField::Submit => FormField::DbAliases,
        };
    }

    pub fn validate(&mut self) -> bool {
        self.validation_error = None;

        if self.name.trim().is_empty() {
            self.validation_error = Some("Connection name cannot be empty".to_string());
            return false;
        }

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

        true
    }

    pub fn to_connection_config(&self, db_aliases: HashMap<u32, String>) -> ConnectionConfig {
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
            sni: if self.sni.is_empty() {
                None
            } else {
                Some(self.sni.clone())
            },
            db_aliases,
        }
    }

    pub fn from_connection_config(config: &ConnectionConfig) -> Self {
        Self {
            name: config.name.clone(),
            host: config.host.clone(),
            port: config.port.to_string(),
            password: config.password.clone(),
            username: config.username.clone(),
            use_tls: config.use_tls,
            allow_insecure_tls: config.allow_insecure_tls,
            sni: config.sni.clone().unwrap_or_default(),
            db_aliases: serde_json::to_string(&config.db_aliases)
                .unwrap_or_else(|_| "{\"0\":\"0\"}".to_string()),
            editing_field: FormField::Name,
            validation_error: None,
            editing_connection_id: Some(config.id.clone()),
        }
    }

    // Parse connection string from clipboard and create a ConnectionConfig
    // Supports redis:// and rediss:// URLs, as well as redis-cli command format
    pub fn from_connection_string(input: &str) -> Result<Self, String> {
        let mut form = Self {
            name: String::new(),
            host: String::new(),
            port: String::new(),
            password: None,
            username: None,
            use_tls: false,
            allow_insecure_tls: true,
            sni: String::new(),
            db_aliases: "{\"0\":\"0\"}".to_string(),
            editing_field: FormField::Name,
            validation_error: None,
            editing_connection_id: None,
        };

        Self::parse_connection_string(input, &mut form);

        // Validate parsed data
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

        Ok(form)
    }

    fn parse_connection_string(input: &str, form: &mut Self) {
        let mut url_to_parse = String::new();
        let mut override_host = None;
        let mut override_port = None;
        let mut override_user = None;
        let mut override_pass = None;
        let mut override_tls = false;
        let mut override_sni = None;

        if input.starts_with("redis-cli") {
            let parts: Vec<&str> = input.split_whitespace().collect();
            let mut i = 1;
            while i < parts.len() {
                match parts[i] {
                    "-u" => {
                        if i + 1 < parts.len() {
                            url_to_parse = parts[i + 1].to_string();
                            i += 1;
                        }
                    }
                    "-h" => {
                        if i + 1 < parts.len() {
                            override_host = Some(parts[i + 1].to_string());
                            i += 1;
                        }
                    }
                    "-p" => {
                        if i + 1 < parts.len() {
                            override_port = Some(parts[i + 1].to_string());
                            i += 1;
                        }
                    }
                    "-a" => {
                        if i + 1 < parts.len() {
                            override_pass = Some(parts[i + 1].to_string());
                            i += 1;
                        }
                    }
                    "--user" => {
                        if i + 1 < parts.len() {
                            override_user = Some(parts[i + 1].to_string());
                            i += 1;
                        }
                    }
                    "--tls" => {
                        override_tls = true;
                    }
                    "--sni" => {
                        if i + 1 < parts.len() {
                            override_sni = Some(parts[i + 1].to_string());
                            i += 1;
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
            if let Some(stripped) = url_to_parse.strip_prefix("rediss://") {
                form.use_tls = true;
                Self::parse_authority(stripped, form);
            } else if let Some(stripped) = url_to_parse.strip_prefix("redis://") {
                Self::parse_authority(stripped, form);
            }
        }

        if let Some(h) = override_host {
            form.host = h;
        }
        if let Some(p) = override_port {
            form.port = p;
        }
        if let Some(u) = override_user {
            form.username = Some(u);
        }
        if let Some(pass) = override_pass {
            form.password = Some(pass);
        }
        if override_tls {
            form.use_tls = true;
        }
        if let Some(s) = override_sni {
            form.sni = s;
        }
    }

    fn parse_authority(authority: &str, form: &mut Self) {
        let (user_pass, host_port) = if let Some(idx) = authority.rfind('@') {
            (Some(&authority[..idx]), &authority[idx + 1..])
        } else {
            (None, authority)
        };

        if let Some(up) = user_pass {
            if let Some(idx) = up.find(':') {
                form.username = Some(up[..idx].to_string());
                form.password = Some(up[idx + 1..].to_string());
            } else {
                form.password = Some(up.to_string());
            }
        }

        let host_port_no_db = if let Some(idx) = host_port.find('/') {
            &host_port[..idx]
        } else {
            host_port
        };

        if let Some(idx) = host_port_no_db.rfind(':') {
            form.host = host_port_no_db[..idx].to_string();
            form.port = host_port_no_db[idx + 1..].to_string();
        } else {
            form.host = host_port_no_db.to_string();
            form.port = "6379".to_string();
        }
    }

    fn is_valid_host(host: &str) -> bool {
        // Check if it's a valid IPv4 address
        if host.split('.').count() == 4 && host.split('.').all(|part| part.parse::<u8>().is_ok()) {
            return true;
        }

        // Check if it's a valid hostname (simplified check)
        if host.is_empty() || host.len() > 253 {
            return false;
        }

        // Check for invalid characters
        if host.contains(|c: char| !c.is_ascii_alphanumeric() && c != '.' && c != '-' && c != '_') {
            return false;
        }

        // Must not start or end with hyphen or dot
        if host.starts_with('-')
            || host.ends_with('-')
            || host.starts_with('.')
            || host.ends_with('.')
        {
            return false;
        }

        // Check each label (part between dots)
        for label in host.split('.') {
            if label.is_empty() || label.len() > 63 {
                return false;
            }
            if label.starts_with('-') || label.ends_with('-') {
                return false;
            }
        }

        true
    }
}
