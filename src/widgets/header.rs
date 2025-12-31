use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use super::format::{format_bytes, format_uptime};
use crate::models::ServerInfo;
use crate::theme::get_colors;

#[derive(Debug, Clone)]
pub enum Message {
    UpdateConnectionName(Option<String>),
    UpdateServerInfo(Option<ServerInfo>),
    SetLoadingServerInfo(bool),
    SetConnecting(bool),
}

#[derive(Clone)]
pub struct Header {
    pub connection_name: Option<String>,
    pub server_info: Option<ServerInfo>,
    pub is_loading_server_info: bool,
    pub is_connecting: bool,
}

impl Header {
    pub fn new() -> Self {
        Self {
            connection_name: None,
            server_info: None,
            is_loading_server_info: false,
            is_connecting: false,
        }
    }

    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::UpdateConnectionName(name) => {
                self.connection_name = name;
            }
            Message::UpdateServerInfo(info) => {
                self.server_info = info;
                self.is_loading_server_info = false;
            }
            Message::SetLoadingServerInfo(loading) => {
                self.is_loading_server_info = loading;
            }
            Message::SetConnecting(connecting) => {
                self.is_connecting = connecting;
            }
        }
    }

    pub fn view(&self, frame: &mut Frame, area: Rect) {
        let colors = get_colors();
        let mut spans = vec![];

        if self.is_connecting {
            if let Some(conn_name) = &self.connection_name {
                spans.extend(vec![
                    Span::styled("Connecting to ", Style::default().fg(colors.info)),
                    Span::styled(
                        conn_name.clone(),
                        Style::default()
                            .fg(colors.info)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" ...", Style::default().fg(colors.info)),
                ]);
            } else {
                spans.push(Span::styled(
                    "Connecting...",
                    Style::default().fg(colors.info),
                ));
            }
        } else if let Some(conn_name) = &self.connection_name {
            if let Some(info) = &self.server_info {
                let uptime = format_uptime(info.uptime_seconds);
                let memory = format_bytes(info.used_memory);

                spans.extend(vec![
                    Span::styled("Connection: ", Style::default().fg(colors.text_secondary)),
                    Span::styled(
                        conn_name.clone(),
                        Style::default()
                            .fg(colors.success)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("  |  ", Style::default().fg(colors.text_secondary)),
                    Span::styled("Uptime: ", Style::default().fg(colors.text_secondary)),
                    Span::styled(uptime, Style::default().fg(colors.info_uptime)),
                    Span::styled("  |  ", Style::default().fg(colors.text_secondary)),
                    Span::styled("Clients: ", Style::default().fg(colors.text_secondary)),
                    Span::styled(
                        format!("{}", info.connected_clients),
                        Style::default().fg(colors.info_clients),
                    ),
                    Span::styled("  |  ", Style::default().fg(colors.text_secondary)),
                    Span::styled("Keys: ", Style::default().fg(colors.text_secondary)),
                    Span::styled(
                        format!("{}", info.total_keys),
                        Style::default().fg(colors.info_keys),
                    ),
                    Span::styled("  |  ", Style::default().fg(colors.text_secondary)),
                    Span::styled("Memory: ", Style::default().fg(colors.text_secondary)),
                    Span::styled(memory, Style::default().fg(colors.info_memory)),
                ]);
            } else {
                spans.extend(vec![
                    Span::styled("Connection: ", Style::default().fg(colors.text_secondary)),
                    Span::styled(
                        conn_name.clone(),
                        Style::default()
                            .fg(colors.success)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]);
            }

            if self.is_loading_server_info {
                spans.extend(vec![
                    Span::styled("  |  ", Style::default().fg(colors.text_secondary)),
                    Span::styled("Loading server info...", Style::default().fg(colors.info)),
                ]);
            }
        } else {
            spans.push(Span::styled(
                " Not connected - Please select a connection from the list",
                Style::default().fg(colors.text_secondary),
            ));
        }

        let header = Paragraph::new(Line::from(spans))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(colors.border_default)),
            )
            .alignment(ratatui::layout::Alignment::Left);

        frame.render_widget(header, area);
    }
}
