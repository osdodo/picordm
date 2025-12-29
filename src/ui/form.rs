use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use unicode_width::UnicodeWidthStr;

use crate::app::App;
use crate::connection::FormField;

const ACTIVE_BORDER_COLOR: Color = Color::Rgb(147, 112, 219);
const INACTIVE_BORDER_COLOR: Color = Color::Rgb(80, 90, 110);
const REQUIRED_COLOR: Color = Color::LightRed;

struct TextFieldConfig<'a> {
    title: &'a str,
    value: &'a str,
    is_required: bool,
    is_active: bool,
    placeholder: &'a str,
    hint: Option<&'a str>,
}

struct CheckboxConfig<'a> {
    title: &'a str,
    checked: bool,
    is_active: bool,
}

fn render_text_field(frame: &mut Frame, area: Rect, config: TextFieldConfig) -> Option<(u16, u16)> {
    let border_color = if config.is_active {
        ACTIVE_BORDER_COLOR
    } else {
        INACTIVE_BORDER_COLOR
    };

    let title_color = if config.is_active {
        ACTIVE_BORDER_COLOR
    } else {
        Color::White
    };

    let mut title_spans = vec![Span::styled(
        config.title,
        Style::default()
            .fg(title_color)
            .add_modifier(Modifier::BOLD),
    )];

    if config.is_required {
        title_spans.push(Span::styled(
            " *",
            Style::default()
                .fg(REQUIRED_COLOR)
                .add_modifier(Modifier::BOLD),
        ));
    }

    if let Some(hint) = config.hint {
        title_spans.push(Span::styled(
            format!(" ({})", hint),
            Style::default().fg(title_color),
        ));
    }

    let display_value = if config.value.is_empty() && !config.is_active {
        config.placeholder
    } else {
        config.value
    };

    let value_style = if config.value.is_empty() && !config.is_active {
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM)
    } else {
        Style::default().fg(Color::White)
    };

    let widget = Paragraph::new(display_value).style(value_style).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .title(Line::from(title_spans)),
    );

    frame.render_widget(widget, area);

    // Return cursor position
    if config.is_active {
        Some((area.x + 1 + config.value.width() as u16, area.y + 1))
    } else {
        None
    }
}

fn render_checkbox(frame: &mut Frame, area: Rect, config: CheckboxConfig) {
    let border_color = if config.is_active {
        ACTIVE_BORDER_COLOR
    } else {
        INACTIVE_BORDER_COLOR
    };

    let title_color = if config.is_active {
        ACTIVE_BORDER_COLOR
    } else {
        Color::White
    };

    let title_span = Line::from(vec![Span::styled(
        config.title,
        Style::default()
            .fg(title_color)
            .add_modifier(Modifier::BOLD),
    )]);

    let check_display = if config.checked {
        Line::from(vec![
            Span::styled(
                "✓ ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Enabled", Style::default().fg(Color::Green)),
        ])
    } else {
        Line::from(vec![
            Span::styled("○ ", Style::default().fg(Color::DarkGray)),
            Span::styled("Disabled", Style::default().fg(Color::DarkGray)),
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

pub fn render_connection_form(frame: &mut Frame, app: &mut App, area: Rect) {
    let title = if app.connection_form.editing_connection_id.is_some() {
        "Edit Redis Connection"
    } else {
        "New Redis Connection"
    };

    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            title,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(80, 90, 110)));
    frame.render_widget(block, area);

    let inner_area = area.inner(ratatui::layout::Margin {
        horizontal: 2,
        vertical: 1,
    });

    let _ = render_mode_tabs(frame, app, inner_area);

    // Rendering form fields
    let form_area = Rect {
        x: inner_area.x,
        y: inner_area.y + 2,
        width: inner_area.width,
        height: inner_area.height - 2,
    };

    let cursor_pos = render_form_fields(frame, app, form_area);

    // Set cursor position
    if let Some((x, y)) = cursor_pos {
        frame.set_cursor_position(ratatui::layout::Position { x, y });
    }
}

fn render_mode_tabs(frame: &mut Frame, app: &App, area: Rect) -> Option<(u16, u16)> {
    let tab_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 1,
    };

    let standalone_style = if !app.connection_form.is_cluster {
        Style::default()
            .fg(Color::Rgb(147, 112, 219))
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let cluster_style = if app.connection_form.is_cluster {
        Style::default()
            .fg(Color::Rgb(147, 112, 219))
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let tabs = Line::from(vec![
        Span::styled("[ Standalone ]", standalone_style),
        Span::raw("  "),
        Span::styled("[ Cluster ]", cluster_style),
        Span::raw("  "),
        Span::styled(
            "(Press Tab to switch)",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM),
        ),
    ]);

    frame.render_widget(Paragraph::new(tabs), tab_area);
    None
}

fn render_form_fields(frame: &mut Frame, app: &mut App, area: Rect) -> Option<(u16, u16)> {
    let has_error = app.connection_form.validation_error.is_some();

    let constraints = if has_error {
        vec![
            Constraint::Length(3), // Name
            Constraint::Length(3), // Host/Port or Cluster
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Username/Password
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // TLS options
            Constraint::Length(3), // SNI
            Constraint::Length(3), // DB Aliases
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Error
        ]
    } else {
        vec![
            Constraint::Length(3), // Name
            Constraint::Length(3), // Host/Port or Cluster
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Username/Password
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // TLS options
            Constraint::Length(3), // SNI
            Constraint::Length(3), // DB Aliases
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut cursor_pos = None;
    let mut idx = 0;

    // Connection Name
    if let Some(pos) = render_text_field(
        frame,
        chunks[idx],
        TextFieldConfig {
            title: "Connection Name",
            value: &app.connection_form.name,
            is_required: true,
            is_active: app.connection_form.editing_field == FormField::Name,
            placeholder: "...",
            hint: None,
        },
    ) {
        cursor_pos = Some(pos);
    }
    idx += 1;

    // Host & Port OR Cluster Nodes
    if app.connection_form.is_cluster {
        if let Some(pos) = render_text_field(
            frame,
            chunks[idx],
            TextFieldConfig {
                title: "Cluster Nodes",
                value: &app.connection_form.cluster_nodes,
                is_required: true,
                is_active: app.connection_form.editing_field == FormField::ClusterNodes,
                placeholder: "...",
                hint: Some("e.g. host1:6379, host2:6379"),
            },
        ) {
            cursor_pos = Some(pos);
        }
    } else if let Some(pos) = render_host_port_fields(frame, app, chunks[idx]) {
        cursor_pos = Some(pos);
    }
    idx += 2; // Skip spacer

    // Username & Password
    if let Some(pos) = render_auth_fields(frame, app, chunks[idx]) {
        cursor_pos = Some(pos);
    }
    idx += 2; // Skip spacer

    // TLS Options
    render_tls_fields(frame, app, chunks[idx]);
    idx += 1;

    // SNI
    if let Some(pos) = render_text_field(
        frame,
        chunks[idx],
        TextFieldConfig {
            title: "SNI",
            value: &app.connection_form.sni,
            is_required: false,
            is_active: app.connection_form.editing_field == FormField::Sni,
            placeholder: "...",
            hint: Some("optional"),
        },
    ) {
        cursor_pos = Some(pos);
    }
    idx += 1;

    // DB Aliases
    if let Some(pos) = render_text_field(
        frame,
        chunks[idx],
        TextFieldConfig {
            title: "DB Aliases",
            value: &app.connection_form.db_aliases,
            is_required: false,
            is_active: app.connection_form.editing_field == FormField::DbAliases,
            placeholder: "...",
            hint: Some("optional, JSON"),
        },
    ) {
        cursor_pos = Some(pos);
    }
    idx += 1;

    // Validation Error
    if has_error {
        idx += 1; // Skip spacer
        if let Some(ref error) = app.connection_form.validation_error {
            let error_text = Paragraph::new(Line::from(vec![
                Span::styled(
                    "⚠ ",
                    Style::default()
                        .fg(Color::LightRed)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(error, Style::default().fg(Color::LightRed)),
            ]))
            .alignment(ratatui::layout::Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::LightRed))
                    .style(Style::default().bg(Color::Rgb(50, 20, 20))),
            );
            frame.render_widget(error_text, chunks[idx]);
        }
    }

    cursor_pos
}

fn render_host_port_fields(frame: &mut Frame, app: &App, area: Rect) -> Option<(u16, u16)> {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    let mut cursor_pos = None;

    // Host
    if let Some(pos) = render_text_field(
        frame,
        chunks[0],
        TextFieldConfig {
            title: "Host",
            value: &app.connection_form.host,
            is_required: true,
            is_active: app.connection_form.editing_field == FormField::Host,
            placeholder: "127.0.0.1",
            hint: None,
        },
    ) {
        cursor_pos = Some(pos);
    }

    // Port
    if let Some(pos) = render_text_field(
        frame,
        chunks[1],
        TextFieldConfig {
            title: "Port",
            value: &app.connection_form.port,
            is_required: true,
            is_active: app.connection_form.editing_field == FormField::Port,
            placeholder: "6379",
            hint: None,
        },
    ) {
        cursor_pos = Some(pos);
    }

    cursor_pos
}

fn render_auth_fields(frame: &mut Frame, app: &App, area: Rect) -> Option<(u16, u16)> {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let mut cursor_pos = None;

    // Username
    if let Some(pos) = render_text_field(
        frame,
        chunks[0],
        TextFieldConfig {
            title: "Username",
            value: app.connection_form.username.as_deref().unwrap_or(""),
            is_required: false,
            is_active: app.connection_form.editing_field == FormField::Username,
            placeholder: "...",
            hint: Some("optional"),
        },
    ) {
        cursor_pos = Some(pos);
    }

    // Password
    if let Some(pos) = render_text_field(
        frame,
        chunks[1],
        TextFieldConfig {
            title: "Password",
            value: app.connection_form.password.as_deref().unwrap_or(""),
            is_required: false,
            is_active: app.connection_form.editing_field == FormField::Password,
            placeholder: "...",
            hint: Some("optional"),
        },
    ) {
        cursor_pos = Some(pos);
    }

    cursor_pos
}

fn render_tls_fields(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Use TLS
    render_checkbox(
        frame,
        chunks[0],
        CheckboxConfig {
            title: "Use TLS",
            checked: app.connection_form.use_tls,
            is_active: app.connection_form.editing_field == FormField::UseTls,
        },
    );

    // Allow Insecure TLS
    render_checkbox(
        frame,
        chunks[1],
        CheckboxConfig {
            title: "Allow Insecure TLS",
            checked: app.connection_form.allow_insecure_tls,
            is_active: app.connection_form.editing_field == FormField::AllowInsecureTls,
        },
    );
}
