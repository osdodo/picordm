use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::models::{Screen, ViewMode};
use crate::theme::get_colors;

#[derive(Debug, Clone)]
pub enum Message {
    Screen(Screen),
    ViewMode(ViewMode),
    Error(Option<String>),
}

pub struct Footer {
    pub current_screen: Screen,
    pub current_view_mode: Option<ViewMode>,
    pub error_message: Option<String>,
}

impl Footer {
    pub fn new() -> Self {
        Self {
            current_screen: Screen::Connection,
            current_view_mode: None,
            error_message: None,
        }
    }

    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::Screen(screen) => {
                self.current_screen = screen;
                self.current_view_mode = None;
            }
            Message::ViewMode(view_mode) => {
                self.current_view_mode = Some(view_mode);
            }
            Message::Error(error) => {
                self.error_message = error;
            }
        }
    }

    pub fn view(&self, frame: &mut Frame, area: Rect) {
        if let Some(error) = &self.error_message {
            self.render_error(frame, area, error);
        } else {
            self.render_help(frame, area);
        }
    }

    fn render_error(&self, frame: &mut Frame, area: Rect, error: &str) {
        let colors = get_colors();

        let error_widget = Paragraph::new(Line::from(vec![
            Span::styled("⚠ Error: ", Style::default().fg(colors.error)),
            Span::styled(error, Style::default().fg(colors.text_primary)),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(colors.error)),
        );

        frame.render_widget(error_widget, area);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let colors = get_colors();

        let help_text = if self.current_screen == Screen::Dashboard {
            match self.current_view_mode {
                Some(ViewMode::KeyContent) => {
                    vec![
                        Span::styled("Vim: ", Style::default().fg(colors.cyan)),
                        Span::styled(":w", Style::default().fg(colors.info)),
                        Span::styled("(Save) ", Style::default().fg(colors.text_secondary)),
                        Span::styled(":q", Style::default().fg(colors.info)),
                        Span::styled("(Quit) ", Style::default().fg(colors.text_secondary)),
                        Span::styled(":wq", Style::default().fg(colors.info)),
                        Span::styled("(Save&Quit) | ", Style::default().fg(colors.text_secondary)),
                        Span::styled("i", Style::default().fg(colors.info)),
                        Span::styled("(Insert) ", Style::default().fg(colors.text_secondary)),
                        Span::styled("v", Style::default().fg(colors.info)),
                        Span::styled("(Visual) ", Style::default().fg(colors.text_secondary)),
                        Span::styled("hjkl", Style::default().fg(colors.info)),
                        Span::styled("(Navigate) | ", Style::default().fg(colors.text_secondary)),
                        Span::styled("Ctrl+q", Style::default().fg(colors.info)),
                        Span::styled(": Exit App", Style::default().fg(colors.text_secondary)),
                    ]
                }
                Some(ViewMode::CommandMode) => {
                    vec![
                        Span::styled("Enter", Style::default().fg(colors.info)),
                        Span::styled(": Execute | ", Style::default().fg(colors.text_secondary)),
                        Span::styled("Tab", Style::default().fg(colors.info)),
                        Span::styled(
                            ": Browse Output | ",
                            Style::default().fg(colors.text_secondary),
                        ),
                        Span::styled("Esc", Style::default().fg(colors.info)),
                        Span::styled(
                            ": Exit Command Mode",
                            Style::default().fg(colors.text_secondary),
                        ),
                    ]
                }
                _ => {
                    vec![
                        Span::styled("j/k/↑↓", Style::default().fg(colors.info)),
                        Span::styled(": Navigate | ", Style::default().fg(colors.text_secondary)),
                        Span::styled("Enter", Style::default().fg(colors.info)),
                        Span::styled(": View | ", Style::default().fg(colors.text_secondary)),
                        Span::styled("Space", Style::default().fg(colors.info)),
                        Span::styled(": Select | ", Style::default().fg(colors.text_secondary)),
                        Span::styled("x/Del", Style::default().fg(colors.info)),
                        Span::styled(": Delete | ", Style::default().fg(colors.text_secondary)),
                        Span::styled("/", Style::default().fg(colors.info)),
                        Span::styled(": Search | ", Style::default().fg(colors.text_secondary)),
                        Span::styled(":", Style::default().fg(colors.info)),
                        Span::styled(": Command | ", Style::default().fg(colors.text_secondary)),
                        Span::styled("Ctrl+n", Style::default().fg(colors.info)),
                        Span::styled(": Switch DB | ", Style::default().fg(colors.text_secondary)),
                        Span::styled("Ctrl+r", Style::default().fg(colors.info)),
                        Span::styled(
                            ": Refresh Keys | ",
                            Style::default().fg(colors.text_secondary),
                        ),
                        Span::styled("F5", Style::default().fg(colors.info)),
                        Span::styled(
                            ": Refresh Stats | ",
                            Style::default().fg(colors.text_secondary),
                        ),
                        Span::styled("Ctrl+t", Style::default().fg(colors.info)),
                        Span::styled(
                            ": Switch Conn | ",
                            Style::default().fg(colors.text_secondary),
                        ),
                        Span::styled("Ctrl+b", Style::default().fg(colors.info)),
                        Span::styled(": Disconnect", Style::default().fg(colors.text_secondary)),
                    ]
                }
            }
        } else {
            vec![
                Span::styled("n", Style::default().fg(colors.info)),
                Span::styled(": New | ", Style::default().fg(colors.text_secondary)),
                Span::styled("e", Style::default().fg(colors.info)),
                Span::styled(": Edit | ", Style::default().fg(colors.text_secondary)),
                Span::styled("i", Style::default().fg(colors.info)),
                Span::styled(": Import | ", Style::default().fg(colors.text_secondary)),
                Span::styled("Delete/Backspace", Style::default().fg(colors.info)),
                Span::styled(": Delete | ", Style::default().fg(colors.text_secondary)),
                Span::styled("↑↓", Style::default().fg(colors.info)),
                Span::styled(": Navigate | ", Style::default().fg(colors.text_secondary)),
                Span::styled("Enter", Style::default().fg(colors.info)),
                Span::styled(": Connect | ", Style::default().fg(colors.text_secondary)),
                Span::styled("Ctrl+q", Style::default().fg(colors.info)),
                Span::styled(": Exit App", Style::default().fg(colors.text_secondary)),
            ]
        };

        let help_widget = Paragraph::new(Line::from(help_text))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(colors.inactive_border)),
            )
            .alignment(ratatui::layout::Alignment::Left);

        frame.render_widget(help_widget, area);
    }
}
