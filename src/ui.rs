use std::sync::OnceLock;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as SyntectStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use unicode_width::UnicodeWidthStr;

use crate::app::{App, CurrentScreen};
use crate::connection::FormField;
use crate::file_selector::DirEntry;

// Cache syntect resources for better performance
static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

fn get_syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn get_theme_set() -> &'static ThemeSet {
    THEME_SET.get_or_init(ThemeSet::load_defaults)
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

    // Connection switcher overlay
    if app.current_screen == CurrentScreen::ConnectionSwitcher {
        let connections = app.connection_list.connections();
        // Calculate height: title(1) + borders(2) + help(1) + spacing(1) + list items
        let list_height = (connections.len() as u16).min(12);
        let popup_height = list_height + 5; // borders + title + help + spacing
        let popup_width = 70; // Wider for better readability
        let popup_area = centered_rect_fixed_height(popup_width, popup_height, size);
        frame.render_widget(Clear, popup_area);
        render_connection_switcher(frame, app, popup_area);
    }

    // Delete confirmation dialog
    if app.is_delete_confirmation_open {
        let popup_area = centered_rect_fixed_height(60, 8, size);
        frame.render_widget(Clear, popup_area);
        render_delete_confirmation(frame, app, popup_area);
    }

    // Progress dialog
    if app.progress_dialog.is_some() {
        let popup_area = centered_rect_fixed_height(60, 8, size);
        frame.render_widget(Clear, popup_area);
        render_progress_dialog(frame, app, popup_area);
    }
}

fn render_header(frame: &mut Frame, app: &mut App, area: Rect) {
    let mut header_spans = vec![];

    // Show connecting status with target connection name
    if app.is_connecting {
        if let Some(conn) = app.connection_list.selected_connection() {
            header_spans.extend(vec![
                Span::styled("Connecting to ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    &conn.name,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" ...", Style::default().fg(Color::Yellow)),
            ]);
        } else {
            header_spans.push(Span::styled(
                "Connecting...",
                Style::default().fg(Color::Yellow),
            ));
        }
    } else if let Some(conn_name) = &app.current_connection_name {
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
            Span::styled("Loading...", Style::default().fg(Color::Yellow)),
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

fn format_bytes(bytes: u64, precision: usize, include_gb: bool) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if include_gb && bytes >= GB {
        format!(
            "{:.precision$} GB",
            bytes as f64 / GB as f64,
            precision = precision
        )
    } else if bytes >= MB {
        format!(
            "{:.precision$} MB",
            bytes as f64 / MB as f64,
            precision = precision
        )
    } else if bytes >= KB {
        format!(
            "{:.precision$} KB",
            bytes as f64 / KB as f64,
            precision = precision
        )
    } else {
        format!("{} B", bytes)
    }
}

fn format_memory(bytes: u64) -> String {
    format_bytes(bytes, 2, true)
}

fn format_file_size(bytes: u64) -> String {
    format_bytes(bytes, 1, false)
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
        render_connection_content(frame, app, chunks[1]);
    } else if app.current_screen == CurrentScreen::FileSelector {
        render_file_selector(frame, app, area);
    } else {
        render_key_sidebar(frame, app, chunks[0]);
        render_content_with_command(frame, app, chunks[1]);
    }
}

fn render_connection_content(frame: &mut Frame, app: &App, area: Rect) {
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

    // Display error message if present, otherwise show default prompt
    let content = if let Some(err) = &app.error_message {
        Span::styled(
            format!("Error: {}", err),
            Style::default().fg(Color::Red),
        )
    } else {
        Span::styled(
            "Select a connection to start",
            Style::default().fg(Color::White),
        )
    };

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn render_content_with_command(frame: &mut Frame, app: &mut App, area: Rect) {
    // Render content area directly (command input is integrated inside)
    render_content_area(frame, app, area);
}

fn render_content_area(frame: &mut Frame, app: &mut App, area: Rect) {
    // If there's command output or in command mode, show CLI interface
    let show_cli_interface =
        !app.command_output.is_empty() || app.current_screen == CurrentScreen::CommandMode;

    if show_cli_interface {
        render_cli_ui(frame, app, area);
        return;
    }

    // Otherwise show key value viewer
    let title = if !app.current_value.is_empty() && app.is_json_content {
        "View (Press 'e' to edit | ↑↓ to scroll)"
    } else {
        "View"
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(80, 90, 110)))
        .title(Span::styled(
            title,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));

    if app.is_loading_value
        && (app.current_screen == CurrentScreen::Dashboard
            || app.current_screen == CurrentScreen::KeyContent)
    {
        let loading_text = Span::styled("Loading value...", Style::default().fg(Color::Yellow));
        let paragraph = Paragraph::new(loading_text)
            .block(block)
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(paragraph, area);
    } else if let Some(err) = &app.error_message {
        // Display error messages only in non-CLI mode
        let paragraph = Paragraph::new(format!("Error: {}", err))
            .block(block)
            .wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);
    } else if !app.current_value.is_empty() && app.is_json_content {
        // Clamp scroll offset to valid range
        let inner_height = area.height.saturating_sub(1); // Account for borders
        let content_lines = app.current_value.lines().count() as u16;
        if content_lines > inner_height {
            let max_scroll = content_lines.saturating_sub(inner_height);
            if app.scroll_offset > max_scroll {
                app.scroll_offset = max_scroll;
            }
        } else {
            app.scroll_offset = 0;
        }

        // Display key-value pairs in JSON format
        // Use cached highlighted JSON if available, otherwise generate and cache it
        if app.cached_highlighted_json.is_none() {
            if let Ok(lines) = highlight_json_with_syntect(&app.current_value) {
                app.cached_highlighted_json = Some(lines);
            }
        }

        if let Some(lines) = &app.cached_highlighted_json {
            let paragraph = Paragraph::new(lines.clone())
                .block(block)
                .scroll((app.scroll_offset, 0));
            frame.render_widget(paragraph, area);
        } else {
            let paragraph = Paragraph::new(app.current_value.clone())
                .block(block)
                .scroll((app.scroll_offset, 0))
                .wrap(Wrap { trim: true });
            frame.render_widget(paragraph, area);
        }
    } else if !app.current_value.is_empty() {
        // Clamp scroll offset to valid range
        let inner_height = area.height.saturating_sub(1); // Account for borders
        let content_lines = app.current_value.lines().count() as u16;
        if content_lines > inner_height {
            let max_scroll = content_lines.saturating_sub(inner_height);
            if app.scroll_offset > max_scroll {
                app.scroll_offset = max_scroll;
            }
        } else {
            app.scroll_offset = 0;
        }

        // Display key-value pairs in plain text format
        let paragraph = Paragraph::new(app.current_value.clone())
            .block(block)
            .scroll((app.scroll_offset, 0))
            .wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);
    } else {
        // Default prompt
        let paragraph =
            Paragraph::new("Select a key to view its value or press '>' to execute a command")
                .block(block)
                .wrap(Wrap { trim: true })
                .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(paragraph, area);
    }
}

fn render_json_editor(frame: &mut Frame, app: &mut App, area: Rect) {
    match highlight_json_with_syntect(&app.json_editor.lines().join("\n")) {
        Ok(highlighted_lines) => {
            // Get cursor position from textarea
            let (cursor_row, cursor_col) = app.json_editor.cursor();

            // Add cursor indicator to title
            let title = format!(
                "JSON Editor (Esc: Cancel, Ctrl+s: Save, Ctrl+q: Quit) - Ln {}, Col {}",
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
                        "JSON Editor (Esc: Cancel, Ctrl+s: Save, Ctrl+q: Quit)",
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
    let connection_names: Vec<String> = app
        .connection_list
        .connections()
        .iter()
        .map(|c| c.name.clone())
        .collect();

    let items: Vec<ListItem> = connection_names
        .iter()
        .map(|name| {
            let name_line = Line::from(Span::styled(
                name,
                Style::default().add_modifier(Modifier::BOLD),
            ));

            ListItem::new(name_line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(80, 90, 110)))
                .title(Span::styled(
                    "Connections",
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

fn render_cli_ui(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Command input
            Constraint::Min(1),    // Output display
        ])
        .split(area);

    render_command_input(frame, app, chunks[0]);
    render_command_output(frame, app, chunks[1]);
}

fn render_command_input(frame: &mut Frame, app: &App, area: Rect) {
    let title = if app.current_screen == CurrentScreen::CommandMode {
        "Command Input (Enter: Execute | Esc: Exit)"
    } else {
        "Command Input (Press '>' to enter command mode)"
    };

    let border_color = if app.current_screen == CurrentScreen::CommandMode {
        Color::Rgb(147, 112, 219)
    } else {
        Color::Rgb(80, 90, 110)
    };

    let input_content = if app.current_screen == CurrentScreen::CommandMode {
        format!("> {}", app.command_input)
    } else {
        "> ".to_string()
    };

    let input_widget = Paragraph::new(input_content)
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

    frame.render_widget(input_widget, area);

    // Set cursor position if in command mode
    if app.current_screen == CurrentScreen::CommandMode {
        let cursor_x = area.x + 3 + app.command_input.width() as u16; // "> " + input
        let cursor_y = area.y + 1;

        if cursor_x < area.right() && cursor_y < area.bottom() {
            frame.set_cursor_position(ratatui::layout::Position {
                x: cursor_x,
                y: cursor_y,
            });
        }
    }
}

fn render_command_output(frame: &mut Frame, app: &mut App, area: Rect) {
    let title = if app.command_output.is_empty() {
        "Command Output"
    } else if app.is_json_content {
        "Command Output (↑↓ to scroll)"
    } else {
        "Command Output (↑↓ to scroll)"
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(80, 90, 110)))
        .title(Span::styled(
            title,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));

    if app.command_output.is_empty() {
        let paragraph = Paragraph::new("No command executed yet")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(paragraph, area);
    } else {
        // Reuse the exact same logic as render_content_area for value display
        let inner_height = area.height.saturating_sub(2); // Account for borders
        let content_lines = app.current_value.lines().count() as u16;

        if content_lines > inner_height {
            let max_scroll = content_lines.saturating_sub(inner_height);
            if app.scroll_offset > max_scroll {
                app.scroll_offset = max_scroll;
            }
        } else {
            app.scroll_offset = 0;
        }

        if app.is_json_content {
            // Use cached highlighted JSON if available, otherwise generate and cache it
            if app.cached_highlighted_json.is_none() {
                if let Ok(lines) = highlight_json_with_syntect(&app.current_value) {
                    app.cached_highlighted_json = Some(lines);
                }
            }

            if let Some(lines) = &app.cached_highlighted_json {
                let paragraph = Paragraph::new(lines.clone())
                    .block(block)
                    .scroll((app.scroll_offset, 0));
                frame.render_widget(paragraph, area);
            } else {
                let paragraph = Paragraph::new(app.current_value.clone())
                    .block(block)
                    .scroll((app.scroll_offset, 0))
                    .wrap(Wrap { trim: true });
                frame.render_widget(paragraph, area);
            }
        } else {
            // Display plain text with scrolling
            let paragraph = Paragraph::new(app.current_value.clone())
                .block(block)
                .scroll((app.scroll_offset, 0))
                .wrap(Wrap { trim: true });
            frame.render_widget(paragraph, area);
        }
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
        "Database (Press 'Ctrl+d' to select)"
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

        frame.render_widget(Clear, dropdown_area); // Clear background first
        frame.render_stateful_widget(dropdown_list, dropdown_area, &mut app.db_selector_state);
    }
}

fn render_footer(frame: &mut Frame, app: &mut App, area: Rect) {
    let status_text = match app.current_screen {
        CurrentScreen::NewConnectionForm => "Esc: Cancel | Tab: Next | Enter: Toggle/Save",
        CurrentScreen::ConnectionList => {
            "n: New connection | e: Edit connection | i: Import connection | Delete/Backspace: Delete connection | ↑↓: Nav | Enter: Connect | Ctrl+q: Quit"
        }
        CurrentScreen::Dashboard => {
            if app.is_delete_confirmation_open {
                "Y: Confirm Delete | N/Esc: Cancel"
            } else {
                match (app.is_db_selector_open, app.is_searching_keys) {
                    (true, _) => "Esc: Close | ↑↓: Nav | Enter: Select Database",
                    (_, true) => {
                        "Esc: Exit Search | Space: Toggle | Ctrl+a: Select/Clear All | Enter: Select | Arrow: Navigate"
                    }
                    _ if !app.current_value.is_empty() && app.is_json_content => {
                        "e: Edit JSON | b: Back | ↑↓: Scroll | Enter: View Key | /: Search | >: Command | Ctrl+t: Switch Connection | Ctrl+e: Export | Ctrl+l: Import"
                    }
                    _ if !app.selected_keys.is_empty() => {
                        "Space: Toggle | Ctrl+a: Select/Clear All | Delete/Backspace: Delete | Enter: View | /: Search | >: Command | Ctrl+t: Switch Connection | Ctrl+e: Export | Ctrl+l: Import"
                    }
                    _ => {
                        "Space: Select | b: Back | Ctrl+a: Select All | Enter: View | /: Search | >: Command | Ctrl+d: Switch DB | Ctrl+r: Refresh | Ctrl+t: Switch Connection | Ctrl+e: Export | Ctrl+l: Import"
                    }
                }
            }
        }
        CurrentScreen::KeyContent => {
            "b: Back to Keys | e: Edit JSON | ↑↓: Scroll | Ctrl+t: Switch Connection | Ctrl+e: Export | Ctrl+l: Import | Ctrl+q: Quit"
        }
        CurrentScreen::JsonEditor => "Esc: Cancel | Ctrl+s: Save | Ctrl+q: Quit",
        CurrentScreen::CommandMode => "Enter: Execute | ↑↓: Scroll | Esc: Exit Command Mode",
        CurrentScreen::FileSelector => "↑↓: Navigate | Enter: Import | Esc: Cancel",
        CurrentScreen::ConnectionSwitcher => {
            "↑↓: Navigate | Type: Search | 1-9: Quick Select | Enter: Switch | Esc: Cancel"
        }
    };

    let status = format!(" {}", status_text);
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
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(80, 90, 110)));
    frame.render_widget(block, area);

    // Track cursor position for active field
    let mut cursor_pos: Option<(u16, u16)> = None;

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

    let active_border_color = Color::Rgb(147, 112, 219); // Medium Purple
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
        let title_color = if is_active {
            Color::Rgb(147, 112, 219)
        } else {
            Color::White
        };
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

        // Set cursor position if this field is active
        if is_active {
            cursor_pos = Some((
                chunks[chunk_idx].x + 1 + value.width() as u16,
                chunks[chunk_idx].y + 1,
            ));
        }

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
        let title_color = if is_active {
            Color::Rgb(147, 112, 219)
        } else {
            Color::White
        };
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

        // Set cursor position if this field is active
        if is_active {
            cursor_pos = Some((
                host_port_chunks[0].x + 1 + value.width() as u16,
                host_port_chunks[0].y + 1,
            ));
        }

        // Render Port
        let is_active = app.connection_form.editing_field == FormField::Port;
        let border_color = if is_active {
            active_border_color
        } else {
            inactive_border_color
        };
        let title_color = if is_active {
            Color::Rgb(147, 112, 219)
        } else {
            Color::White
        };
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

        // Set cursor position if this field is active
        if is_active {
            cursor_pos = Some((
                host_port_chunks[1].x + 1 + value.width() as u16,
                host_port_chunks[1].y + 1,
            ));
        }

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
        let title_color = if is_active {
            Color::Rgb(147, 112, 219)
        } else {
            Color::White
        };
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

        // Set cursor position if this field is active
        if is_active {
            cursor_pos = Some((
                user_pass_chunks[0].x + 1 + value.width() as u16,
                user_pass_chunks[0].y + 1,
            ));
        }

        // Render Password
        let is_active = app.connection_form.editing_field == FormField::Password;
        let border_color = if is_active {
            active_border_color
        } else {
            inactive_border_color
        };
        let title_color = if is_active {
            Color::Rgb(147, 112, 219)
        } else {
            Color::White
        };
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

        // Set cursor position if this field is active
        if is_active {
            cursor_pos = Some((
                user_pass_chunks[1].x + 1 + value.width() as u16,
                user_pass_chunks[1].y + 1,
            ));
        }

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
        let title_color = if is_active {
            Color::Rgb(147, 112, 219)
        } else {
            Color::White
        };

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
        let title_color = if is_active {
            Color::Rgb(147, 112, 219)
        } else {
            Color::White
        };

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
        let title_color = if is_active {
            Color::Rgb(147, 112, 219)
        } else {
            Color::White
        };
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

        // Set cursor position if this field is active
        if is_active {
            cursor_pos = Some((
                chunks[chunk_idx].x + 1 + value.width() as u16,
                chunks[chunk_idx].y + 1,
            ));
        }

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
        let title_color = if is_active {
            Color::Rgb(147, 112, 219)
        } else {
            Color::White
        };
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

        // Set cursor position if this field is active
        if is_active {
            cursor_pos = Some((
                chunks[chunk_idx].x + 1 + value.width() as u16,
                chunks[chunk_idx].y + 1,
            ));
        }

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
        (Color::Rgb(147, 112, 219), Color::Rgb(147, 112, 219))
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

    // Render cursor for active text input field
    if let Some((x, y)) = cursor_pos {
        frame.set_cursor_position(ratatui::layout::Position { x, y });
    }
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

fn render_connection_switcher(frame: &mut Frame, app: &App, area: Rect) {
    let total_connections = app.connection_list.connections().len();
    let filtered_connections = app.get_filtered_connections();
    let is_filtering = !app.connection_switcher_search.is_empty();

    let title = if is_filtering {
        format!(
            "⚡ Quick Connection Switch ({}/{} connections)",
            filtered_connections.len(),
            total_connections
        )
    } else {
        format!(
            "⚡ Quick Connection Switch ({} connections)",
            total_connections
        )
    };

    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            title,
            Style::default()
                .fg(Color::Rgb(147, 112, 219))
                .add_modifier(Modifier::BOLD),
        )]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(147, 112, 219)))
        .style(Style::default().bg(Color::Rgb(30, 30, 40)));
    frame.render_widget(block, area);

    // Split area for search, list and help text
    let inner = area.inner(Margin {
        horizontal: 2,
        vertical: 1,
    });
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Search box
            Constraint::Min(1),    // List area
            Constraint::Length(1), // Help text
        ])
        .split(inner);

    // Render search box
    let search_text = if is_filtering {
        Span::styled(
            format!("Search: {}", app.connection_switcher_search),
            Style::default().fg(Color::White),
        )
    } else {
        Span::styled("Type to search...", Style::default().fg(Color::DarkGray))
    };
    frame.render_widget(Paragraph::new(Line::from(vec![search_text])), chunks[0]);

    // Build connection list items
    let items: Vec<ListItem> = filtered_connections
        .iter()
        .map(|(original_idx, conn)| {
            let is_current = app
                .current_connection_name
                .as_ref()
                .map(|name| name == &conn.name)
                .unwrap_or(false);

            let name_style = if is_current {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let mut spans = vec![];

            // Number prefix - show for all connections when not filtering
            // But only 1-9 support quick select via number keys
            if !is_filtering {
                let num_str = format!("{:2} ", original_idx + 1);
                let num_style = if *original_idx < 9 {
                    // 1-9: Highlight to indicate quick select support
                    Style::default().fg(Color::DarkGray)
                } else {
                    // 10+: Dimmed to indicate no quick select
                    Style::default().fg(Color::Rgb(60, 60, 60))
                };
                spans.push(Span::styled(num_str, num_style));
            } else {
                spans.push(Span::styled("   ", Style::default()));
            }

            // Current connection indicator
            spans.push(if is_current {
                Span::styled("● ", Style::default().fg(Color::Green))
            } else {
                Span::styled("  ", Style::default())
            });

            // Connection name with optional highlighting
            if is_filtering {
                let filter_lower = app.connection_switcher_search.to_lowercase();
                let name_lower = conn.name.to_lowercase();

                if let Some(pos) = name_lower.find(&filter_lower) {
                    spans.push(Span::styled(&conn.name[..pos], name_style));
                    spans.push(Span::styled(
                        &conn.name[pos..pos + app.connection_switcher_search.len()],
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                    ));
                    spans.push(Span::styled(
                        &conn.name[pos + app.connection_switcher_search.len()..],
                        name_style,
                    ));
                } else {
                    spans.push(Span::styled(&conn.name, name_style));
                }
            } else {
                spans.push(Span::styled(&conn.name, name_style));
            }

            // Connection details
            spans.push(Span::styled(
                format!(" ({}:{})", conn.host, conn.port),
                Style::default().fg(Color::DarkGray),
            ));

            ListItem::new(Line::from(spans))
        })
        .collect();

    // Render list
    let list_widget = if items.is_empty() {
        List::new(vec![ListItem::new(Line::from(Span::styled(
            "No matching connections",
            Style::default().fg(Color::Yellow),
        )))])
    } else {
        List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(147, 112, 219))
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ")
    };

    // Create a temporary ListState for rendering
    // The connection_switcher_state stores original indices, but we need filtered indices
    let mut render_state = ListState::default();
    if let Some(selected_original_idx) = app.connection_switcher_state.selected() {
        // Find the position of the selected item in the filtered list
        let filtered_pos = filtered_connections
            .iter()
            .position(|(idx, _)| *idx == selected_original_idx);
        render_state.select(filtered_pos);
    }

    frame.render_stateful_widget(list_widget, chunks[1], &mut render_state);

    // Build help text
    let purple = Color::Rgb(147, 112, 219);
    let gray = Color::DarkGray;
    let selected_idx = app.connection_switcher_state.selected().unwrap_or(0);

    let help_spans = if is_filtering {
        vec![
            Span::styled(
                "↑↓",
                Style::default().fg(purple).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Navigate  ", Style::default().fg(gray)),
            Span::styled(
                "Backspace",
                Style::default().fg(purple).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Clear  ", Style::default().fg(gray)),
            Span::styled(
                "Enter",
                Style::default().fg(purple).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Switch  ", Style::default().fg(gray)),
            Span::styled(
                "Esc",
                Style::default().fg(purple).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Cancel", Style::default().fg(gray)),
        ]
    } else if total_connections > 9 {
        vec![
            Span::styled(
                "↑↓",
                Style::default().fg(purple).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Nav  ", Style::default().fg(gray)),
            Span::styled(
                "1-9",
                Style::default().fg(purple).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Quick  ", Style::default().fg(gray)),
            Span::styled(
                "Type",
                Style::default().fg(purple).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Search  ", Style::default().fg(gray)),
            Span::styled(
                "Enter",
                Style::default().fg(purple).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Switch  ", Style::default().fg(gray)),
            Span::styled(
                "Esc",
                Style::default().fg(purple).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Cancel  ", Style::default().fg(gray)),
            Span::styled(
                format!("[{}/{}]", selected_idx + 1, total_connections),
                Style::default().fg(Color::Yellow),
            ),
        ]
    } else {
        vec![
            Span::styled(
                "↑↓",
                Style::default().fg(purple).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Navigate  ", Style::default().fg(gray)),
            Span::styled(
                "1-9",
                Style::default().fg(purple).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Quick select  ", Style::default().fg(gray)),
            Span::styled(
                "Type",
                Style::default().fg(purple).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Search  ", Style::default().fg(gray)),
            Span::styled(
                "Enter",
                Style::default().fg(purple).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Switch  ", Style::default().fg(gray)),
            Span::styled(
                "Esc",
                Style::default().fg(purple).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Cancel", Style::default().fg(gray)),
        ]
    };

    frame.render_widget(
        Paragraph::new(Line::from(help_spans)).alignment(ratatui::layout::Alignment::Center),
        chunks[2],
    );
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
            Constraint::Length(2), // Message
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
        Line::from(Span::styled(
            "This action cannot be undone.",
            Style::default().fg(Color::Yellow),
        )),
    ])
    .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(message_widget, chunks[0]);

    let buttons_hint = Paragraph::new(Line::from(vec![
        Span::styled(
            "[y]",
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Yes, delete  ", Style::default().fg(Color::White)),
        Span::styled(
            "[n/Esc]",
            Style::default()
                .fg(Color::Rgb(147, 112, 219))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Cancel", Style::default().fg(Color::White)),
    ]))
    .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(buttons_hint, chunks[1]);
}

fn render_progress_dialog(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(ref dialog) = app.progress_dialog {
        let title_color = if dialog.is_complete {
            Color::Green
        } else {
            Color::Rgb(147, 112, 219)
        };

        let block = Block::default()
            .title(Line::from(vec![Span::styled(
                &dialog.title,
                Style::default()
                    .fg(title_color)
                    .add_modifier(Modifier::BOLD),
            )]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(title_color))
            .style(Style::default().bg(Color::Rgb(25, 25, 35)));

        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(2), // Message with more space
                Constraint::Length(1), // Progress indicator
            ])
            .split(area);

        // Message with icon and better styling
        let message_lines = if dialog.is_complete {
            vec![
                Line::from(""),
                Line::from(vec![Span::styled(
                    &dialog.message,
                    Style::default()
                        .fg(
                            if dialog.message.contains("Exported")
                                || dialog.message.contains("Imported")
                            {
                                Color::Green
                            } else {
                                Color::Red
                            },
                        )
                        .add_modifier(Modifier::BOLD),
                )]),
            ]
        } else {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    &dialog.message,
                    Style::default().fg(Color::White),
                )),
            ]
        };

        let message_widget =
            Paragraph::new(message_lines).alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(message_widget, chunks[0]);

        // Bottom hint - only show when complete
        if dialog.is_complete {
            let hint_widget = Paragraph::new(Line::from(vec![
                Span::styled("Press ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "Esc",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" to close", Style::default().fg(Color::DarkGray)),
            ]))
            .alignment(ratatui::layout::Alignment::Center);
            frame.render_widget(hint_widget, chunks[1]);
        }
    }
}

fn render_file_selector(frame: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect_fixed_height(70, 18, area);
    frame.render_widget(Clear, popup_area);

    // Show current directory in title
    let current_dir_display = app
        .file_selector
        .current_dir
        .to_string_lossy()
        .chars()
        .take(40)
        .collect::<String>();
    let title = format!("Browse Files - {}", current_dir_display);

    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            title,
            Style::default()
                .fg(Color::Rgb(147, 112, 219))
                .add_modifier(Modifier::BOLD),
        )]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(147, 112, 219)))
        .style(Style::default().bg(Color::Rgb(25, 25, 35)));

    frame.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Min(1),    // File list
            Constraint::Length(3), // Instructions
        ])
        .split(popup_area);

    // Directory and file list
    if app.file_selector.dir_entries.is_empty() {
        let no_files_widget = Paragraph::new("Directory is empty or cannot be accessed")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(no_files_widget, chunks[0]);
    } else {
        let items: Vec<ListItem> = app
            .file_selector
            .dir_entries
            .iter()
            .map(|entry| match entry {
                DirEntry::Parent => ListItem::new(Line::from(vec![
                    Span::styled("[DIR] ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        "..",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])),
                DirEntry::Directory(path) => {
                    let dirname = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    ListItem::new(Line::from(vec![
                        Span::styled("[DIR] ", Style::default().fg(Color::Cyan)),
                        Span::styled(dirname, Style::default().fg(Color::Cyan)),
                    ]))
                }
                DirEntry::JsonFile(path, size) => {
                    let filename = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let size_str = format_file_size(*size);
                    ListItem::new(Line::from(vec![
                        Span::styled(filename, Style::default().fg(Color::White)),
                        Span::styled(
                            format!(" ({})", size_str),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]))
                }
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(50, 50, 70))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, chunks[0], &mut app.file_selector.state.clone());
    }

    // Instructions
    let instructions = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                "↑↓",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Navigate  ", Style::default().fg(Color::White)),
            Span::styled(
                "Enter",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Open/Import  ", Style::default().fg(Color::White)),
            Span::styled(
                "Esc",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Cancel", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("[DIR] Directory  ", Style::default().fg(Color::Cyan)),
            Span::styled("JSON File", Style::default().fg(Color::Green)),
        ]),
    ])
    .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(instructions, chunks[1]);
}
