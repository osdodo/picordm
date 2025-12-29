use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
};
use unicode_width::UnicodeWidthStr;

use crate::app::App;

pub fn render_key_sidebar(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search input box
            Constraint::Min(1),    // Key list
            Constraint::Length(3), // DB selector
        ])
        .split(area);

    let cursor_pos = render_search_box(frame, app, chunks[0]);
    render_keys_list(frame, app, chunks[1]);
    render_db_selector(frame, app, chunks[2]);

    // Set cursor position if searching
    if let Some((x, y)) = cursor_pos {
        frame.set_cursor_position(ratatui::layout::Position { x, y });
    }
}

fn render_search_box(frame: &mut Frame, app: &App, area: Rect) -> Option<(u16, u16)> {
    let search_border_color = if app.is_searching_keys {
        Color::Rgb(147, 112, 219)
    } else {
        Color::Rgb(80, 90, 110)
    };

    let search_title = if app.is_searching_keys {
        "Search Keys (Esc to exit)"
    } else {
        "Search Keys (Press '/' to search)"
    };

    let search_display = if app.key_search_filter.is_empty() && !app.is_searching_keys {
        Span::styled(
            "...",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM),
        )
    } else {
        Span::styled(&app.key_search_filter, Style::default().fg(Color::White))
    };

    let search_input = Paragraph::new(Line::from(vec![search_display])).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(search_border_color))
            .title(Span::styled(
                search_title,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
    );

    frame.render_widget(search_input, area);

    // Return cursor position if actively searching
    if app.is_searching_keys {
        Some((
            area.x + 1 + app.key_search_filter.width() as u16,
            area.y + 1,
        ))
    } else {
        None
    }
}

fn render_keys_list(frame: &mut Frame, app: &mut App, area: Rect) {
    if app.is_loading_keys {
        let loading_text = Span::styled("Loading keys...", Style::default().fg(Color::Yellow));
        let loading_widget = Paragraph::new(loading_text)
            .alignment(ratatui::layout::Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(80, 90, 110)))
                    .title(Span::styled(
                        "Keys",
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    )),
            );
        frame.render_widget(loading_widget, area);
    } else {
        let filtered_keys = app.get_filtered_keys();
        let items: Vec<ListItem> = filtered_keys
            .iter()
            .map(|key| {
                let is_selected = app.selected_keys.contains(key);
                let checkbox = if is_selected {
                    Span::styled(
                        "[✓] ",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled("[ ] ", Style::default().fg(Color::DarkGray))
                };

                let key_style = if is_selected {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(Line::from(vec![checkbox, Span::styled(key, key_style)]))
            })
            .collect();

        let selected_count = app.selected_keys.len();
        let keys_title = if selected_count > 0 {
            format!(
                "Keys ({}) - {} selected",
                filtered_keys.len(),
                selected_count
            )
        } else {
            format!("Keys ({})", filtered_keys.len())
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(80, 90, 110)))
                    .title(Span::styled(
                        keys_title,
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(34, 36, 64))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, area, &mut app.key_list_state);
    }
}

fn render_db_selector(frame: &mut Frame, app: &mut App, area: Rect) {
    let display_text = if app.db_list.is_empty() {
        format!("db{}", app.current_db_index)
    } else {
        let current_db_keys = app
            .db_list
            .iter()
            .find(|db| db.index == app.current_db_index)
            .map(|db| db.keys_count)
            .unwrap_or(0);
        format!("db{} - {} keys", app.current_db_index, current_db_keys)
    };

    let border_color = if app.is_db_selector_open {
        Color::Rgb(147, 112, 219)
    } else {
        Color::Rgb(80, 90, 110)
    };

    let title = if app.is_db_selector_open {
        "Database (Esc to close | ↑↓ to navigate | Enter to select)"
    } else {
        "Database (Press 'Ctrl+n' to select)"
    };

    let selector_widget = Paragraph::new(display_text)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .title(Span::styled(
                    title,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )),
        );

    frame.render_widget(selector_widget, area);

    if app.is_db_selector_open && !app.db_list.is_empty() {
        // Calculate dropdown position and size
        let dropdown_height = (app.db_list.len() as u16 + 2).min(10);
        let dropdown_y = if area.y >= dropdown_height {
            area.y.saturating_sub(dropdown_height)
        } else {
            0
        };
        let dropdown_area = Rect {
            x: area.x,
            y: dropdown_y,
            width: area.width,
            height: dropdown_height,
        };

        let items: Vec<ListItem> = app
            .db_list
            .iter()
            .map(|db| {
                let display = format!("db{} ({} keys)", db.index, db.keys_count);
                let style = if db.index == app.current_db_index {
                    Style::default()
                        .fg(Color::Green)
                        .bg(Color::Rgb(30, 30, 40))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White).bg(Color::Rgb(30, 30, 40))
                };
                ListItem::new(display).style(style)
            })
            .collect();

        let dropdown_list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(147, 112, 219)))
                    .style(Style::default().bg(Color::Rgb(30, 30, 40))),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(50, 50, 70))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        frame.render_widget(Clear, dropdown_area);
        frame.render_stateful_widget(dropdown_list, dropdown_area, &mut app.db_selector_state);
    }
}
