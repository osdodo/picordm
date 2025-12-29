use edtui::{EditorMode, EditorTheme, EditorView, SyntaxHighlighter};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

use crate::app::{App, CurrentScreen};
use crate::ui::{cli, dashboard, dialogs, file_selector, form, utils};

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
    render_main(frame, app, chunks[1]);
    render_footer(frame, app, chunks[2]);

    // Overlays
    if app.current_screen == CurrentScreen::NewConnectionForm {
        let form_height = if app.connection_form.validation_error.is_some() {
            30
        } else {
            27
        };
        let popup_area = centered_rect_fixed_height(60, form_height, size);
        frame.render_widget(Clear, popup_area);
        form::render_connection_form(frame, app, popup_area);
    }

    // Connection switcher overlay
    if app.current_screen == CurrentScreen::ConnectionSwitcher {
        let connections = app.connection_list.connections();
        let list_height = (connections.len() as u16).min(12);
        let popup_height = list_height + 5;
        let popup_width = 70;
        let popup_area = centered_rect_fixed_height(popup_width, popup_height, size);
        frame.render_widget(Clear, popup_area);
        dialogs::render_connection_switcher(frame, app, popup_area);
    }

    // Delete confirmation dialog
    if app.is_delete_confirmation_open {
        let popup_area = centered_rect_fixed_height(60, 8, size);
        frame.render_widget(Clear, popup_area);
        dialogs::render_delete_confirmation(frame, app, popup_area);
    }

    // Progress dialog
    if app.progress_dialog.is_some() {
        let popup_area = centered_rect_fixed_height(60, 8, size);
        frame.render_widget(Clear, popup_area);
        dialogs::render_progress_dialog(frame, app, popup_area);
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
            let memory = utils::format_memory(info.used_memory);
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
            Span::styled("Loading server info...", Style::default().fg(Color::Yellow)),
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

fn render_main(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // Sidebar (Connections or Keys)
            Constraint::Percentage(70), // Content (Value)
        ])
        .split(area);

    // Sidebar
    if app.current_screen == CurrentScreen::ConnectionList
        || app.current_screen == CurrentScreen::NewConnectionForm
    {
        render_connection_list(frame, app, chunks[0]);
        render_connection_content(frame, app, chunks[1]);
    } else if app.current_screen == CurrentScreen::FileSelector {
        file_selector::render_file_selector(frame, app, area);
    } else {
        dashboard::render_key_sidebar(frame, app, chunks[0]);
        render_content_area(frame, app, chunks[1]);
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
        Span::styled(format!("Error: {}", err), Style::default().fg(Color::Red))
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

fn render_content_area(frame: &mut Frame, app: &mut App, area: Rect) {
    // If there's command output or in command mode, show CLI interface
    let show_cli_interface =
        !app.command_output.is_empty() || app.current_screen == CurrentScreen::CommandMode;

    if show_cli_interface {
        cli::render_cli_ui(frame, app, area);
        return;
    }

    // Otherwise show key value viewer using edtui (Vim-style editing)
    let title = if app.is_vim_command_mode {
        "Vim Command Mode (Enter to execute, Esc to cancel)"
    } else if !app.current_value.is_empty() {
        "View/Edit (Vim: :w=Save :q=Quit :wq=Save&Quit)"
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
    } else if !app.current_value.is_empty() {
        // Use edtui for both viewing and editing
        if app.is_vim_command_mode {
            // Split area to show editor and command line
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),    // Editor
                    Constraint::Length(1), // Command line
                ])
                .split(area.inner(Margin::new(1, 1)));

            // Render editor without block, with optional syntax highlighting
            let mut editor_view = EditorView::new(&mut app.editor_state);

            // Add JSON syntax highlighting if content is JSON
            if app.is_json_content {
                let syntax_highlighter = SyntaxHighlighter::new("visual-studio-dark", "json");
                editor_view = editor_view.syntax_highlighter(Some(syntax_highlighter));
            }

            frame.render_widget(editor_view, chunks[0]);

            // Render Vim command line
            let cmd_line = format!(":{}", app.vim_command_input);
            let cmd_paragraph = Paragraph::new(cmd_line).style(Style::default().fg(Color::Yellow));
            frame.render_widget(cmd_paragraph, chunks[1]);

            // Render block border
            frame.render_widget(block, area);

            // Set cursor position at end of command input
            let cursor_x = area.x + 2 + app.vim_command_input.len() as u16;
            let cursor_y = area.bottom() - 2;
            if cursor_x < area.right() && cursor_y < area.bottom() {
                frame.set_cursor_position(ratatui::layout::Position {
                    x: cursor_x,
                    y: cursor_y,
                });
            }
        } else {
            // Normal editor view with optional syntax highlighting
            let theme = EditorTheme {
                block: Some(block),
                ..Default::default()
            };

            let mut editor_view = EditorView::new(&mut app.editor_state).theme(theme);

            // Add JSON syntax highlighting if content is JSON
            if app.is_json_content {
                let syntax_highlighter = SyntaxHighlighter::new("visual-studio-dark", "json");
                editor_view = editor_view.syntax_highlighter(Some(syntax_highlighter));
            }

            frame.render_widget(editor_view, area);
        }
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

fn render_footer(frame: &mut Frame, app: &mut App, area: Rect) {
    let status_text = match app.current_screen {
        CurrentScreen::NewConnectionForm => {
            "Esc: Cancel | ↑↓: Navigate | Tab: Switch Mode | Enter: Toggle | Ctrl+S: Save"
        }
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
                        "e: Edit JSON | ↑↓: Scroll | Enter: View Key | /: Search | >: Command | Ctrl+t: Switch Connection | Ctrl+b: Disconnect | Ctrl+e: Export | Ctrl+l: Import"
                    }
                    _ if !app.selected_keys.is_empty() => {
                        "Space: Toggle | Ctrl+a: Select/Clear All | Delete/Backspace: Delete | Enter: View Value | /: Search | >: Command | Ctrl+b: Disconnect | Ctrl+t: Switch Connection | Ctrl+e: Export | Ctrl+l: Import"
                    }
                    _ => {
                        "Space: Select | Enter: View Value | Ctrl+a: Select All | /: Search | >: Command | Ctrl+n: Switch DB | F5: Refresh Server Stats | Ctrl+t: Switch Connection | Ctrl+b: Disconnect | Ctrl+e: Export | Ctrl+l: Import"
                    }
                }
            }
        }
        CurrentScreen::KeyContent => {
            "Vim: :w(Save) :q(Quit) :wq(Save&Quit) | i(Insert) v(Visual) hjkl(Navigate) | Ctrl+q: Exit App"
        }
        CurrentScreen::CommandMode => {
            if app.command_mode_focus_on_output {
                match app.editor_state.mode {
                    EditorMode::Visual => "y: Copy | d: Delete | Esc: Exit Visual Mode",
                    EditorMode::Insert => "Type to edit | Esc: Exit Insert Mode",
                    EditorMode::Normal => {
                        "hjkl/Arrows: Navigate | v: Visual | i: Insert | Tab: Back to Input | Esc: Exit Command Mode"
                    }
                    EditorMode::Search => "Type to search | Esc: Exit Search Mode",
                }
            } else {
                "Enter: Execute | Tab: Browse Output | Esc: Exit Command Mode"
            }
        }
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
