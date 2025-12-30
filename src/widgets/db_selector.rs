use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent},
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph},
};

use crate::models::DbInfo;

#[derive(Debug, Clone)]
pub enum Message {
    Close,
    Next,
    Previous,
    Select,
}

#[derive(Debug, Clone)]
pub enum UpdateResult {
    None,
    Selected(u32),
}

pub struct DbSelector {
    pub db_list: Vec<DbInfo>,
    pub current_db_index: u32,
    pub is_open: bool,
    pub state: ListState,
}

impl DbSelector {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            db_list: Vec::new(),
            current_db_index: 0,
            is_open: false,
            state,
        }
    }

    pub fn handle_key_events(&self, key: KeyEvent) -> Option<Message> {
        if !self.is_open {
            return None;
        }
        match key.code {
            KeyCode::Esc => Some(Message::Close),
            KeyCode::Char('j') | KeyCode::Down => Some(Message::Next),
            KeyCode::Char('k') | KeyCode::Up => Some(Message::Previous),
            KeyCode::Enter => Some(Message::Select),
            _ => None,
        }
    }

    pub fn update(&mut self, msg: Message) -> UpdateResult {
        match msg {
            Message::Close => {
                self.toggle();
                UpdateResult::None
            }
            Message::Next => {
                self.next();
                UpdateResult::None
            }
            Message::Previous => {
                self.previous();
                UpdateResult::None
            }
            Message::Select => {
                if let Some(selected) = self.state.selected() {
                    if let Some(db_info) = self.db_list.get(selected) {
                        let db_index = db_info.index;
                        self.toggle();
                        self.current_db_index = db_index;
                        UpdateResult::Selected(db_index)
                    } else {
                        UpdateResult::None
                    }
                } else {
                    UpdateResult::None
                }
            }
        }
    }

    pub fn view(&mut self, frame: &mut Frame, area: Rect) {
        let display_text = if self.db_list.is_empty() {
            format!("db{}", self.current_db_index)
        } else {
            let current_db_keys = self
                .db_list
                .iter()
                .find(|db| db.index == self.current_db_index)
                .map(|db| db.keys_count)
                .unwrap_or(0);
            format!("db{} - {} keys", self.current_db_index, current_db_keys)
        };

        let border_color = if self.is_open {
            Color::Rgb(147, 112, 219)
        } else {
            Color::Rgb(80, 90, 110)
        };

        let title = if self.is_open {
            "Database (Esc to close | ↑↓ to navigate | Enter to select)"
        } else {
            "Database (Press 'Ctrl+n' to select)"
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

        if self.is_open && !self.db_list.is_empty() {
            self.render_dropdown(frame, area);
        }
    }

    fn render_dropdown(&mut self, frame: &mut Frame, area: Rect) {
        let dropdown_height = (self.db_list.len() as u16 + 2).min(10);
        let dropdown_y = if area.y >= dropdown_height {
            area.y.saturating_sub(dropdown_height)
        } else {
            0
        };
        let dropdown_area = Rect {
            x: area.x,
            y: dropdown_y,
            width: area.width,
            height: dropdown_height,
        };

        let items: Vec<ListItem> = self
            .db_list
            .iter()
            .map(|db| {
                let display = format!("db{} ({} keys)", db.index, db.keys_count);
                let style = if db.index == self.current_db_index {
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

        frame.render_widget(Clear, dropdown_area);
        frame.render_stateful_widget(dropdown_list, dropdown_area, &mut self.state);
    }

    pub fn next(&mut self) {
        if self.db_list.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.db_list.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.db_list.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.db_list.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
        if self.is_open && !self.db_list.is_empty() {
            // Select current database when opening
            if let Some(index) = self
                .db_list
                .iter()
                .position(|db| db.index == self.current_db_index)
            {
                self.state.select(Some(index));
            } else {
                self.state.select(Some(0));
            }
        }
    }

    pub fn update_db_list(&mut self, db_list: Vec<DbInfo>) {
        self.db_list = db_list;
    }
}
