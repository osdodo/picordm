pub fn format_bytes(bytes: u64, precision: usize, include_gb: bool) -> String {
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

pub fn format_memory(bytes: u64) -> String {
    format_bytes(bytes, 2, true)
}

pub fn format_file_size(bytes: u64) -> String {
    format_bytes(bytes, 1, false)
}

use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub fn centered_rect_fixed_height(percent_x: u16, height: u16, r: Rect) -> Rect {
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
