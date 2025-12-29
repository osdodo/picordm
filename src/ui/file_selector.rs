use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
};

use crate::app::App;
use crate::file_selector::DirEntry;
use crate::ui::utils::format_file_size;

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

pub fn render_file_selector(frame: &mut Frame, app: &App, area: Rect) {
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
