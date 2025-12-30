use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use super::format::{format_bytes, format_uptime};
use crate::models::ServerInfo;

#[derive(Debug, Clone)]
pub enum Message {
    UpdateConnectionName(Option<String>),
    UpdateServerInfo(Option<ServerInfo>),
    SetLoadingServerInfo(bool),
}

#[derive(Clone)]
pub struct Header {
    pub connection_name: Option<String>,
    pub server_info: Option<ServerInfo>,
    pub is_loading_server_info: bool,
}

impl Header {
    pub fn new() -> Self {
        Self {
            connection_name: None,
            server_info: None,
            is_loading_server_info: false,
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
        }
    }

    pub fn view(&self, frame: &mut Frame, area: Rect, is_connecting: bool) {
        let mut spans = vec![];

        if is_connecting {
            if let Some(conn_name) = &self.connection_name {
                spans.extend(vec![
                    Span::styled("Connecting to ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        conn_name.clone(),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" ...", Style::default().fg(Color::Yellow)),
                ]);
            } else {
                spans.push(Span::styled(
                    "Connecting...",
                    Style::default().fg(Color::Yellow),
                ));
            }
        } else if let Some(conn_name) = &self.connection_name {
            if let Some(info) = &self.server_info {
                let uptime = format_uptime(info.uptime_seconds);
                let memory = format_bytes(info.used_memory);

                spans.extend(vec![
                    Span::styled("Connection: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        conn_name.clone(),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  |  "),
                    Span::styled("Uptime: ", Style::default().fg(Color::Gray)),
                    Span::styled(uptime, Style::default().fg(Color::Yellow)),
                    Span::raw("  |  "),
                    Span::styled("Clients: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{}", info.connected_clients),
                        Style::default().fg(Color::Rgb(147, 112, 219)),
                    ),
                    Span::raw("  |  "),
                    Span::styled("Keys: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{}", info.total_keys),
                        Style::default().fg(Color::Magenta),
                    ),
                    Span::raw("  |  "),
                    Span::styled("Memory: ", Style::default().fg(Color::Gray)),
                    Span::styled(memory, Style::default().fg(Color::Green)),
                ]);
            } else {
                spans.extend(vec![
                    Span::styled("Connection: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        conn_name.clone(),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]);
            }

            if self.is_loading_server_info {
                spans.extend(vec![
                    Span::raw("  |  "),
                    Span::styled("Loading server info...", Style::default().fg(Color::Yellow)),
                ]);
            }
        } else {
            spans.push(Span::styled(
                "Not connected - Please select a connection from the list",
                Style::default().fg(Color::DarkGray),
            ));
        }

        let header = Paragraph::new(Line::from(spans))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(80, 90, 110))),
            )
            .alignment(ratatui::layout::Alignment::Left);

        frame.render_widget(header, area);
    }
}
