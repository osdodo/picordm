use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::models::{Screen, ViewMode};

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
        let error_widget = Paragraph::new(Line::from(vec![
            Span::styled("⚠ Error: ", Style::default().fg(Color::LightRed)),
            Span::styled(error, Style::default().fg(Color::White)),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::LightRed)),
        );

        frame.render_widget(error_widget, area);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help_text = if self.current_screen == Screen::Dashboard {
            match self.current_view_mode {
                Some(ViewMode::KeyContent) => {
                    vec![
                        Span::styled("Vim: ", Style::default().fg(Color::Yellow)),
                        Span::styled(":w", Style::default().fg(Color::Cyan)),
                        Span::raw("(Save) "),
                        Span::styled(":q", Style::default().fg(Color::Cyan)),
                        Span::raw("(Quit) "),
                        Span::styled(":wq", Style::default().fg(Color::Cyan)),
                        Span::raw("(Save&Quit) | "),
                        Span::styled("i", Style::default().fg(Color::Cyan)),
                        Span::raw("(Insert) "),
                        Span::styled("v", Style::default().fg(Color::Cyan)),
                        Span::raw("(Visual) "),
                        Span::styled("hjkl", Style::default().fg(Color::Cyan)),
                        Span::raw("(Navigate) | "),
                        Span::styled("Ctrl+q", Style::default().fg(Color::Cyan)),
                        Span::raw(": Exit App"),
                    ]
                }
                Some(ViewMode::CommandMode) => {
                    vec![
                        Span::styled("Enter", Style::default().fg(Color::Cyan)),
                        Span::raw(": Execute | "),
                        Span::styled("Tab", Style::default().fg(Color::Cyan)),
                        Span::raw(": Browse Output | "),
                        Span::styled("Esc", Style::default().fg(Color::Cyan)),
                        Span::raw(": Exit Command Mode"),
                    ]
                }
                _ => {
                    vec![
                        Span::styled("j/k/↑↓", Style::default().fg(Color::Cyan)),
                        Span::raw(": Navigate | "),
                        Span::styled("Enter", Style::default().fg(Color::Cyan)),
                        Span::raw(": View | "),
                        Span::styled("Space", Style::default().fg(Color::Cyan)),
                        Span::raw(": Select | "),
                        Span::styled("x/Del", Style::default().fg(Color::Cyan)),
                        Span::raw(": Delete | "),
                        Span::styled("/", Style::default().fg(Color::Cyan)),
                        Span::raw(": Search | "),
                        Span::styled(":", Style::default().fg(Color::Cyan)),
                        Span::raw(": Command | "),
                        Span::styled("Ctrl+n", Style::default().fg(Color::Cyan)),
                        Span::raw(": Switch DB | "),
                        Span::styled("Ctrl+r", Style::default().fg(Color::Cyan)),
                        Span::raw(": Refresh Keys | "),
                        Span::styled("F5", Style::default().fg(Color::Cyan)),
                        Span::raw(": Refresh Stats | "),
                        Span::styled("Ctrl+t", Style::default().fg(Color::Cyan)),
                        Span::raw(": Switch Conn | "),
                        Span::styled("Ctrl+b", Style::default().fg(Color::Cyan)),
                        Span::raw(": Disconnect"),
                    ]
                }
            }
        } else {
            vec![
                Span::styled("n", Style::default().fg(Color::Cyan)),
                Span::raw(": New | "),
                Span::styled("e", Style::default().fg(Color::Cyan)),
                Span::raw(": Edit | "),
                Span::styled("i", Style::default().fg(Color::Cyan)),
                Span::raw(": Import | "),
                Span::styled("Delete/Backspace", Style::default().fg(Color::Cyan)),
                Span::raw(": Delete | "),
                Span::styled("↑↓", Style::default().fg(Color::Cyan)),
                Span::raw(": Navigate | "),
                Span::styled("Enter", Style::default().fg(Color::Cyan)),
                Span::raw(": Connect | "),
                Span::styled("Ctrl+q", Style::default().fg(Color::Cyan)),
                Span::raw(": Exit App"),
            ]
        };

        let help_widget = Paragraph::new(Line::from(help_text))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(80, 90, 110))),
            )
            .alignment(ratatui::layout::Alignment::Left);

        frame.render_widget(help_widget, area);
    }
}
