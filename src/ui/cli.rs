use edtui::{EditorMode, EditorTheme, EditorView};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Paragraph},
};
use unicode_width::UnicodeWidthStr;

use crate::app::{App, CurrentScreen};

pub fn render_cli_ui(frame: &mut Frame, app: &mut App, area: Rect) {
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
        if app.command_mode_focus_on_output {
            "Command Input (Tab: Switch to Input | Esc: Exit)"
        } else {
            "Command Input (Enter: Execute | Tab: Browse Output | Esc: Exit)"
        }
    } else {
        "Command Input (Press '>' to enter command mode)"
    };

    let border_color = if app.current_screen == CurrentScreen::CommandMode {
        if app.command_mode_focus_on_output {
            Color::Rgb(80, 90, 110) // Dimmed when not focused
        } else {
            Color::Rgb(147, 112, 219) // Highlighted when focused
        }
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

    // Set cursor position if in command mode and focus is on input
    if app.current_screen == CurrentScreen::CommandMode && !app.command_mode_focus_on_output {
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
    } else if app.command_mode_focus_on_output {
        match app.editor_state.mode {
            EditorMode::Visual => {
                "Command Output (Visual Mode - Esc to exit Visual, then Esc to exit Command Mode)"
            }
            EditorMode::Insert => {
                "Command Output (Insert Mode - Esc to exit Insert, then Esc to exit Command Mode)"
            }
            EditorMode::Normal => {
                "Command Output (Browsing - hjkl/arrows to navigate, Tab to return)"
            }
            EditorMode::Search => "Command Output (Search Mode - Esc to exit Search)",
        }
    } else {
        "Command Output (Tab to browse)"
    };

    let border_color = if app.command_mode_focus_on_output {
        Color::Rgb(147, 112, 219) // Highlighted when focused
    } else {
        Color::Rgb(80, 90, 110) // Normal when not focused
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
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
        // Use edtui to display command output
        let theme = EditorTheme {
            block: Some(block),
            ..Default::default()
        };
        let editor_view = EditorView::new(&mut app.editor_state).theme(theme);
        frame.render_widget(editor_view, area);
    }
}
