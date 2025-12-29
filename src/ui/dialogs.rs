use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
};

use crate::app::App;

pub fn render_delete_confirmation(frame: &mut Frame, app: &App, area: Rect) {
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

pub fn render_progress_dialog(frame: &mut Frame, app: &App, area: Rect) {
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

pub fn render_connection_switcher(frame: &mut Frame, app: &App, area: Rect) {
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
    let inner = area.inner(ratatui::layout::Margin {
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

            // Number prefix
            if !is_filtering {
                let num_str = format!("{:2} ", original_idx + 1);
                let num_style = if *original_idx < 9 {
                    Style::default().fg(Color::DarkGray)
                } else {
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
    let mut render_state = ListState::default();
    if let Some(selected_original_idx) = app.connection_switcher_state.selected() {
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
