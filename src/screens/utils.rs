use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::Block,
};

use crate::theme::get_colors;

pub fn centered_rect_fixed_size(width: u16, height: u16, r: Rect) -> Rect {
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
            Constraint::Min(0),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(popup_layout[1])[1]
}

pub fn render_background(frame: &mut Frame, area: Rect) {
    let bg_color = get_colors().bg_main;
    // If the background color is transparent, ignore it.
    if bg_color == Color::Reset {
        return;
    }

    frame.render_widget(Block::default().style(Style::default().bg(bg_color)), area);
}
