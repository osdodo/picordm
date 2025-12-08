use std::sync::OnceLock;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as SyntectStyle, ThemeSet};
use syntect::parsing::SyntaxSet;

use crate::app::{App, CurrentScreen};
use crate::connection::FormField;

// Cache syntect resources for better performance
static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

fn get_syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn get_theme_set() -> &'static ThemeSet {
    THEME_SET.get_or_init(ThemeSet::load_defaults)
}

fn get_spinner_frame(f: usize) -> &'static str {
    const BRAILLE_SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    BRAILLE_SPINNER[f % BRAILLE_SPINNER.len()]
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let size = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(1),    // Main content
            Constraint::Length(3), // Footer
        ])
        .split(size);

    render_header(frame, app, chunks[0]);

    if app.current_screen == CurrentScreen::JsonEditor {
        render_json_editor(frame, app, chunks[1]);
    } else {
        render_main(frame, app, chunks[1]);
    }

    render_footer(frame, app, chunks[2]);

    // Overlays
    if app.current_screen == CurrentScreen::NewConnectionForm {
        // Calculate form height: top(1) + name(3) + host_port(3) + spacer(1) +
        // username_password(3) + spacer(1) + tls(3) + sni(3) + db_aliases(3) +
        // spacer(1) + submit(3) + bottom(1) + margin(2*2) = 30 or 33
        let form_height = if app.connection_form.validation_error.is_some() {
            33
        } else {
            30
        };
        let popup_area = centered_rect_fixed_height(60, form_height, size);
        frame.render_widget(Clear, popup_area);
        render_new_connection_form(frame, app, popup_area);
    }

    // Delete confirmation dialog
    if app.is_delete_confirmation_open {
        let popup_area = centered_rect_fixed_height(50, 10, size);
        frame.render_widget(Clear, popup_area);
        render_delete_confirmation(frame, app, popup_area);
    }
}

fn render_header(frame: &mut Frame, app: &mut App, area: Rect) {
    let mut header_spans = vec![];

    if let Some(conn_name) = &app.current_connection_name {
        if let Some(info) = &app.server_info {
            let uptime = format_uptime(info.uptime_seconds);
            let memory = format_memory(info.used_memory);
            header_spans.extend(vec![
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
                    Style::default().fg(Color::Cyan),
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
                Span::raw("  |  "),
                Span::styled(
                    "[Ctrl+R]",
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Refresh", Style::default().fg(Color::Gray)),
            ]);
        } else {
            header_spans.extend(vec![
                Span::styled("Connection: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    conn_name.clone(),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  |  "),
                Span::styled("Loading server info...", Style::default().fg(Color::Gray)),
                Span::raw("  "),
                Span::styled(
                    "[Ctrl+R]",
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Refresh", Style::default().fg(Color::Gray)),
            ]);
        }
    } else {
        header_spans.push(Span::styled(
            "Not connected - Please select a connection from the list",
            Style::default().fg(Color::DarkGray),
        ));
    }

    // loading indicator
    if app.is_loading_server_info {
        header_spans.extend(vec![
            Span::raw("  |  "),
            Span::styled("Loading ", Style::default().fg(Color::Yellow)),
            Span::styled(
                get_spinner_frame(app.loading_frame),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
    }

    let header_content = Line::from(header_spans);

    let paragraph = Paragraph::new(header_content).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(80, 90, 110))),
    );

    frame.render_widget(paragraph, area);
}

fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;

    if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

fn format_memory(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn render_main(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // Sidebar (Connections or Keys)
            Constraint::Percentage(70), // Content (Value)
        ])
        .split(area);

    // Sidebar
    if app.current_screen == CurrentScreen::ConnectionList {
        render_connection_list(frame, app, chunks[0]);
    } else {
        render_key_sidebar(frame, app, chunks[0]);
    }

    // Content: Value Viewer with scrolling and syntax highlighting
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
    let block = if app.is_json_content && app.current_screen == CurrentScreen::KeyContent {
        block.title(Span::styled(
            "View (Press 'e' to edit | j/k to scroll)",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ))
    } else {
        block
    };

    if app.is_loading_value
        && (app.current_screen == CurrentScreen::Dashboard
            || app.current_screen == CurrentScreen::KeyContent)
    {
        let loading_text = Line::from(vec![
            Span::styled(
                get_spinner_frame(app.loading_frame),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled("Loading value...", Style::default().fg(Color::Yellow)),
        ]);
        let paragraph = Paragraph::new(loading_text)
            .block(block)
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(paragraph, chunks[1]);
    } else if let Some(err) = &app.error_message {
        let paragraph = Paragraph::new(format!("Error: {}", err))
            .block(block)
            .wrap(Wrap { trim: true });
        frame.render_widget(paragraph, chunks[1]);
    } else if app.current_screen == CurrentScreen::ConnectionList {
        let paragraph = Paragraph::new("Select a connection to start")
            .block(block)
            .wrap(Wrap { trim: true });
        frame.render_widget(paragraph, chunks[1]);
    } else if app.is_json_content {
        match highlight_json_with_syntect(&app.current_value) {
            Ok(lines) => {
                let paragraph = Paragraph::new(lines)
                    .block(block)
                    .scroll((app.scroll_offset, 0));
                frame.render_widget(paragraph, chunks[1]);
            }
            Err(_) => {
                let paragraph = Paragraph::new(app.current_value.clone())
                    .block(block)
                    .scroll((app.scroll_offset, 0))
                    .wrap(Wrap { trim: true });
                frame.render_widget(paragraph, chunks[1]);
            }
        }
    } else {
        let paragraph = Paragraph::new(app.current_value.clone())
            .block(block)
            .scroll((app.scroll_offset, 0))
            .wrap(Wrap { trim: true });
        frame.render_widget(paragraph, chunks[1]);
    }
}

fn render_json_editor(frame: &mut Frame, app: &mut App, area: Rect) {
    match highlight_json_with_syntect(&app.json_editor.lines().join("\n")) {
        Ok(highlighted_lines) => {
            // Get cursor position from textarea
            let (cursor_row, cursor_col) = app.json_editor.cursor();

            // Add cursor indicator to title
            let title = format!(
                "JSON Editor (Esc: Cancel, Ctrl+s: Save, q: Quit) - Ln {}, Col {}",
                cursor_row + 1,
                cursor_col + 1
            );

            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(80, 90, 110)))
                .title(Span::styled(
                    title,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().fg(Color::White));

            let paragraph = Paragraph::new(highlighted_lines).block(block);

            frame.render_widget(paragraph, area);

            // Draw cursor - calculate cursor position in the rendered area
            let x = area.x + 1 + (cursor_col as u16).min(area.width.saturating_sub(3));
            let y = area.y + 1 + (cursor_row as u16).min(area.height.saturating_sub(3));

            frame.set_cursor_position(ratatui::layout::Position { x, y });
        }
        Err(_) => {
            app.json_editor.set_block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(80, 90, 110)))
                    .title(Span::styled(
                        "JSON Editor (Esc: Cancel, Ctrl+s: Save, q: Quit)",
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    )),
            );
            frame.render_widget(&app.json_editor, area);
        }
    }
}

fn highlight_json_with_syntect(
    text: &str,
) -> Result<Vec<Line<'static>>, Box<dyn std::error::Error>> {
    let ps = get_syntax_set();
    let ts = get_theme_set();

    let syntax = ps
        .find_syntax_by_extension("json")
        .ok_or("JSON syntax not found")?;

    let theme = ts
        .themes
        .get("base16-eighties.dark")
        .ok_or("base16-eighties.dark theme not found")?;

    let mut h = HighlightLines::new(syntax, theme);

    let mut highlighted_lines = Vec::new();

    for line in text.lines() {
        let ranges: Vec<(SyntectStyle, &str)> = h.highlight_line(line, ps)?;
        let mut spans = Vec::new();

        for (style, text) in ranges {
            let fg = style.foreground;
            let color = Color::Rgb(fg.r, fg.g, fg.b);

            let mut ratatui_style = Style::default().fg(color);
            if style
                .font_style
                .contains(syntect::highlighting::FontStyle::BOLD)
            {
                ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
            }
            if style
                .font_style
                .contains(syntect::highlighting::FontStyle::ITALIC)
            {
                ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
            }
            if style
                .font_style
                .contains(syntect::highlighting::FontStyle::UNDERLINE)
            {
                ratatui_style = ratatui_style.add_modifier(Modifier::UNDERLINED);
            }

            spans.push(Span::styled(text.to_string(), ratatui_style));
        }

        highlighted_lines.push(Line::from(spans));
    }

    Ok(highlighted_lines)
}

fn render_connection_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let selected_idx = app.connection_list.selected();
    let is_connecting = app.is_connecting;
    let is_connection_list_screen = app.current_screen == CurrentScreen::ConnectionList;
    let loading_frame = app.loading_frame;

    let connection_names: Vec<String> = app
        .connection_list
        .connections()
        .iter()
        .map(|c| c.name.clone())
        .collect();

    let items: Vec<ListItem> = connection_names
        .iter()
        .enumerate()
        .map(|(idx, name)| {
            let is_selected = selected_idx == Some(idx);
            let name_line = if is_connecting && is_selected && is_connection_list_screen {
                Line::from(vec![
                    Span::styled(
                        get_spinner_frame(loading_frame),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(name, Style::default().add_modifier(Modifier::BOLD)),
                ])
            } else {
                Line::from(Span::styled(
                    name,
                    Style::default().add_modifier(Modifier::BOLD),
                ))
            };

            ListItem::new(name_line)
        })
        .collect();

    let title = if is_connecting && is_connection_list_screen {
        format!(
            "Connections - Connecting {}",
            get_spinner_frame(loading_frame)
        )
    } else {
        "Connections".to_string()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(80, 90, 110)))
                .title(Span::styled(
                    title,
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

    frame.render_stateful_widget(list, area, app.connection_list.state());
}

fn render_key_sidebar(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search input box
            Constraint::Min(1),    // Key list
            Constraint::Length(3), // DB selector
        ])
        .split(area);

    render_search_box(frame, app, chunks[0]);
    render_keys_list(frame, app, chunks[1]);
    render_db_selector(frame, app, chunks[2]);
}

fn render_search_box(frame: &mut Frame, app: &App, area: Rect) {
    let search_border_color = if app.is_searching_keys {
        Color::Cyan
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
}

fn render_keys_list(frame: &mut Frame, app: &mut App, area: Rect) {
    if app.is_loading_keys {
        let loading_text = Line::from(vec![
            Span::styled(
                get_spinner_frame(app.loading_frame),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled("Loading keys...", Style::default().fg(Color::Yellow)),
        ]);
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

                ListItem::new(Line::from(vec![
                    checkbox,
                    Span::styled(key, key_style),
                ]))
            })
            .collect();

        let selected_count = app.selected_keys.len();
        let keys_title = if selected_count > 0 {
            format!("Keys ({}) - {} selected", filtered_keys.len(), selected_count)
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
        Color::Cyan
    } else {
        Color::Rgb(80, 90, 110)
    };

    let title = if app.is_db_selector_open {
        "Database (Esc to close | ↑↓ to navigate | Enter to select)"
    } else {
        "Database (Press 'd' to select)"
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
        let dropdown_height = (app.db_list.len() as u16 + 2).min(10); // Max 10 items visible
        // Position dropdown above the selector since selector is at the bottom
        let dropdown_y = if area.y >= dropdown_height {
            area.y.saturating_sub(dropdown_height)
        } else {
            0
        };
        let dropdown_area = Rect {
            x: area.x,
            y: dropdown_y, // Above the selector
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
                    .border_style(Style::default().fg(Color::Cyan))
                    .style(Style::default().bg(Color::Rgb(30, 30, 40))),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(50, 50, 70))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        frame.render_widget(Clear, dropdown_area); // Clear background first
        frame.render_stateful_widget(dropdown_list, dropdown_area, &mut app.db_selector_state);
    }
}

fn render_footer(frame: &mut Frame, app: &mut App, area: Rect) {
    let status_text = match app.current_screen {
        CurrentScreen::NewConnectionForm => "Esc: Cancel | Tab: Next | Enter: Toggle/Save",
        CurrentScreen::ConnectionList => {
            "n: New | e: Edit | i: Import | Delete/Backspace: Delete | j/k: Nav | Enter: Connect | q: Quit"
        }
        CurrentScreen::Dashboard => {
            if app.is_delete_confirmation_open {
                "Y: Confirm Delete | N/Esc: Cancel"
            } else if app.is_db_selector_open {
                "Esc: Close | j/k: Nav | Enter: Select Database"
            } else if app.is_searching_keys {
                "Esc: Exit Search | Enter: Select | Arrow: Navigate"
            } else if !app.selected_keys.is_empty() {
                "Space: Toggle | a: Select All | Ctrl+a: Clear | x: Delete | Enter: View | /: Search"
            } else {
                "Space: Select | a: Select All | Enter: View | /: Search | d: DB | Ctrl+r: Refresh | b: Back"
            }
        }
        CurrentScreen::KeyContent => "b: Back to Keys | e: Edit JSON | j/k: Scroll | q: Quit",
        CurrentScreen::JsonEditor => "Esc: Cancel | Ctrl+s: Save | q: Quit",
    };

    let status = format!("Mode: {:?} | {}", app.current_screen, status_text);
    let paragraph = Paragraph::new(status)
        .style(Style::default().fg(Color::Gray))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(80, 90, 110))),
        );
    frame.render_widget(paragraph, area);
}

fn render_new_connection_form(frame: &mut Frame, app: &mut App, area: Rect) {
    let title = if app.connection_form.editing_connection_id.is_some() {
        "Edit Redis Connection"
    } else {
        "New Redis Connection"
    };

    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(80, 90, 110)));
    frame.render_widget(block, area);

    let constraints = if app.connection_form.validation_error.is_some() {
        vec![
            Constraint::Length(1), // Top spacing
            Constraint::Length(3), // Name *
            Constraint::Length(3), // Host & Port (horizontal)
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Username & Password (horizontal)
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Use TLS & Allow Insecure TLS (horizontal)
            Constraint::Length(3), // SNI
            Constraint::Length(3), // DB Aliases
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Error message
            Constraint::Length(3), // Submit button
            Constraint::Length(1), // Bottom spacing
        ]
    } else {
        vec![
            Constraint::Length(1), // Top spacing
            Constraint::Length(3), // Name *
            Constraint::Length(3), // Host & Port (horizontal)
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Username & Password (horizontal)
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Use TLS & Allow Insecure TLS (horizontal)
            Constraint::Length(3), // SNI
            Constraint::Length(3), // DB Aliases
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Submit button
            Constraint::Length(1), // Bottom spacing
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(constraints)
        .split(area);

    let mut chunk_idx = 1; // Skip first spacing chunk

    let active_border_color = Color::Cyan;
    let inactive_border_color = Color::Rgb(80, 90, 110);
    let required_color = Color::LightRed;

    // Connection Name (required)
    {
        let is_active = app.connection_form.editing_field == FormField::Name;
        let border_color = if is_active {
            active_border_color
        } else {
            inactive_border_color
        };
        let title_color = if is_active { Color::Cyan } else { Color::White };
        let value = &app.connection_form.name;

        let title_span = Line::from(vec![
            Span::styled(
                "Connection Name",
                Style::default()
                    .fg(title_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " *",
                Style::default()
                    .fg(required_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);

        let display_value = if value.is_empty() && !is_active {
            "..."
        } else {
            value.as_str()
        };

        let widget = Paragraph::new(display_value)
            .style(if value.is_empty() && !is_active {
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM)
            } else {
                Style::default().fg(Color::White)
            })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(border_color))
                    .title(title_span),
            );
        frame.render_widget(widget, chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // Host & Port (required) - horizontal layout
    {
        let host_port_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[chunk_idx]);

        // Render Host
        let is_active = app.connection_form.editing_field == FormField::Host;
        let border_color = if is_active {
            active_border_color
        } else {
            inactive_border_color
        };
        let title_color = if is_active { Color::Cyan } else { Color::White };
        let value = &app.connection_form.host;

        let title_span = Line::from(vec![
            Span::styled(
                "Host",
                Style::default()
                    .fg(title_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " *",
                Style::default()
                    .fg(required_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);

        let widget = Paragraph::new(value.as_str())
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(border_color))
                    .title(title_span),
            );
        frame.render_widget(widget, host_port_chunks[0]);

        // Render Port
        let is_active = app.connection_form.editing_field == FormField::Port;
        let border_color = if is_active {
            active_border_color
        } else {
            inactive_border_color
        };
        let title_color = if is_active { Color::Cyan } else { Color::White };
        let value = &app.connection_form.port;

        let title_span = Line::from(vec![
            Span::styled(
                "Port",
                Style::default()
                    .fg(title_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " *",
                Style::default()
                    .fg(required_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);

        let widget = Paragraph::new(value.as_str())
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(border_color))
                    .title(title_span),
            );
        frame.render_widget(widget, host_port_chunks[1]);
        chunk_idx += 2; // Skip spacer
    }

    // Username & Password (optional) - horizontal layout
    {
        let user_pass_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[chunk_idx]);

        // Render Username
        let is_active = app.connection_form.editing_field == FormField::Username;
        let border_color = if is_active {
            active_border_color
        } else {
            inactive_border_color
        };
        let title_color = if is_active { Color::Cyan } else { Color::White };
        let value = app.connection_form.username.as_deref().unwrap_or("");

        let title_span = Line::from(vec![
            Span::styled("Username", Style::default().fg(title_color)),
            Span::styled(" (optional)", Style::default().fg(title_color)),
        ]);

        let display_value = if value.is_empty() && !is_active {
            "..."
        } else {
            value
        };

        let widget = Paragraph::new(display_value)
            .style(if value.is_empty() && !is_active {
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM)
            } else {
                Style::default().fg(Color::White)
            })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(border_color))
                    .title(title_span),
            );
        frame.render_widget(widget, user_pass_chunks[0]);

        // Render Password
        let is_active = app.connection_form.editing_field == FormField::Password;
        let border_color = if is_active {
            active_border_color
        } else {
            inactive_border_color
        };
        let title_color = if is_active { Color::Cyan } else { Color::White };
        let value = app.connection_form.password.as_deref().unwrap_or("");

        let title_span = Line::from(vec![
            Span::styled("Password", Style::default().fg(title_color)),
            Span::styled(" (optional)", Style::default().fg(title_color)),
        ]);

        let display_value = if value.is_empty() && !is_active {
            "..."
        } else {
            value
        };

        let widget = Paragraph::new(display_value)
            .style(if value.is_empty() && !is_active {
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM)
            } else {
                Style::default().fg(Color::White)
            })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(border_color))
                    .title(title_span),
            );
        frame.render_widget(widget, user_pass_chunks[1]);
        chunk_idx += 2; // Skip spacer
    }

    // Use TLS & Allow Insecure TLS - horizontal layout
    {
        let tls_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[chunk_idx]);

        // Use TLS
        let is_active = app.connection_form.editing_field == FormField::UseTls;
        let checked = app.connection_form.use_tls;
        let border_color = if is_active {
            active_border_color
        } else {
            inactive_border_color
        };
        let title_color = if is_active { Color::Cyan } else { Color::White };

        let title_span = Line::from(vec![Span::styled(
            "Use TLS",
            Style::default().fg(title_color),
        )]);

        let check_display = if checked {
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
        frame.render_widget(widget, tls_chunks[0]);

        // Allow Insecure TLS
        let is_active = app.connection_form.editing_field == FormField::AllowInsecureTls;
        let checked = app.connection_form.allow_insecure_tls;
        let border_color = if is_active {
            active_border_color
        } else {
            inactive_border_color
        };
        let title_color = if is_active { Color::Cyan } else { Color::White };

        let title_span = Line::from(vec![Span::styled(
            "Allow Insecure TLS",
            Style::default().fg(title_color),
        )]);

        let check_display = if checked {
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
        frame.render_widget(widget, tls_chunks[1]);
        chunk_idx += 1;
    }

    // SNI (optional)
    {
        let is_active = app.connection_form.editing_field == FormField::Sni;
        let border_color = if is_active {
            active_border_color
        } else {
            inactive_border_color
        };
        let title_color = if is_active { Color::Cyan } else { Color::White };
        let value = &app.connection_form.sni;

        let title_span = Line::from(vec![
            Span::styled("SNI", Style::default().fg(title_color)),
            Span::styled(" (optional)", Style::default().fg(title_color)),
        ]);

        let display_value = if value.is_empty() && !is_active {
            "..."
        } else {
            value.as_str()
        };

        let widget = Paragraph::new(display_value)
            .style(if value.is_empty() && !is_active {
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM)
            } else {
                Style::default().fg(Color::White)
            })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(border_color))
                    .title(title_span),
            );
        frame.render_widget(widget, chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // DB Aliases (optional)
    {
        let is_active = app.connection_form.editing_field == FormField::DbAliases;
        let border_color = if is_active {
            active_border_color
        } else {
            inactive_border_color
        };
        let title_color = if is_active { Color::Cyan } else { Color::White };
        let value = &app.connection_form.db_aliases;

        let title_span = Line::from(vec![
            Span::styled("DB Aliases", Style::default().fg(title_color)),
            Span::styled(" (optional, JSON)", Style::default().fg(title_color)),
        ]);

        let display_value = if value.is_empty() && !is_active {
            "..."
        } else {
            value.as_str()
        };

        let widget = Paragraph::new(display_value)
            .style(if value.is_empty() && !is_active {
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM)
            } else {
                Style::default().fg(Color::White)
            })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(border_color))
                    .title(title_span),
            );
        frame.render_widget(widget, chunks[chunk_idx]);
        chunk_idx += 2; // Skip spacer
    }

    // validation error
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
        frame.render_widget(error_text, chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // Submit button
    let button_area = {
        let button_width = 12; // Width of the button (smaller for "Save")
        let parent_width = chunks[chunk_idx].width;
        let x_offset = if parent_width > button_width {
            (parent_width - button_width) / 2
        } else {
            0
        };

        Rect {
            x: chunks[chunk_idx].x + x_offset,
            y: chunks[chunk_idx].y,
            width: button_width.min(parent_width),
            height: chunks[chunk_idx].height,
        }
    };

    let is_submit_focused = app.connection_form.editing_field == FormField::Submit;
    let (submit_fg, submit_border) = if is_submit_focused {
        (Color::Cyan, Color::Cyan)
    } else {
        (Color::DarkGray, Color::Rgb(80, 90, 110))
    };

    let submit_content = Line::from(vec![Span::styled(
        "Save",
        Style::default()
            .fg(submit_fg)
            .add_modifier(if is_submit_focused {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
    )]);

    let submit_btn = Paragraph::new(submit_content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(submit_border)),
        )
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(submit_btn, button_area);
}

#[allow(dead_code)]
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn centered_rect_fixed_height(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_delete_confirmation(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            "⚠ Confirm Delete",
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        )]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::LightRed))
        .style(Style::default().bg(Color::Rgb(30, 30, 40)));

    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Message
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Buttons hint
        ])
        .split(area);

    let selected_count = app.selected_keys.len();
    let message = if selected_count == 1 {
        "Are you sure you want to delete 1 key?".to_string()
    } else {
        format!("Are you sure you want to delete {} keys?", selected_count)
    };

    let message_widget = Paragraph::new(vec![
        Line::from(Span::styled(
            message,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "This action cannot be undone.",
            Style::default().fg(Color::Yellow),
        )),
    ])
    .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(message_widget, chunks[0]);

    let buttons_hint = Paragraph::new(Line::from(vec![
        Span::styled(
            "[Y]",
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Yes, delete  ", Style::default().fg(Color::White)),
        Span::styled(
            "[N/Esc]",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Cancel", Style::default().fg(Color::White)),
    ]))
    .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(buttons_hint, chunks[2]);
}
