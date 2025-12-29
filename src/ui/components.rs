use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use unicode_width::UnicodeWidthStr;

use crate::connection::FormField;

pub const ACTIVE_BORDER_COLOR: Color = Color::Rgb(147, 112, 219);
pub const INACTIVE_BORDER_COLOR: Color = Color::Rgb(80, 90, 110);
pub const REQUIRED_COLOR: Color = Color::LightRed;

pub struct TextFieldConfig<'a> {
    pub title: &'a str,
    pub value: &'a str,
    pub is_required: bool,
    pub is_active: bool,
    pub placeholder: &'a str,
    pub hint: Option<&'a str>,
}

pub fn render_text_field(
    frame: &mut Frame,
    area: Rect,
    config: TextFieldConfig,
) -> Option<(u16, u16)> {
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

    // Return to cursor position
    if config.is_active {
        Some((area.x + 1 + config.value.width() as u16, area.y + 1))
    } else {
        None
    }
}

pub struct CheckboxConfig<'a> {
    pub title: &'a str,
    pub checked: bool,
    pub is_active: bool,
}

pub fn render_checkbox(frame: &mut Frame, area: Rect, config: CheckboxConfig) {
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

/// Get the value reference of the field
#[allow(dead_code)]
pub fn get_field_value(form: &crate::connection::ConnectionForm, field: FormField) -> &str {
    match field {
        FormField::Name => &form.name,
        FormField::Host => &form.host,
        FormField::Port => &form.port,
        FormField::Username => form.username.as_deref().unwrap_or(""),
        FormField::Password => form.password.as_deref().unwrap_or(""),
        FormField::Sni => &form.sni,
        FormField::ClusterNodes => &form.cluster_nodes,
        FormField::DbAliases => &form.db_aliases,
        FormField::UseTls | FormField::AllowInsecureTls => "",
    }
}
