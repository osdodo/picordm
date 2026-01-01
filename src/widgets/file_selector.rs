use std::path::PathBuf;

use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
};

use crate::theme::get_colors;
use crate::widgets::format::format_bytes;

#[derive(Debug, Clone)]
pub enum DirEntry {
    Parent,
    Directory(PathBuf),
    JsonFile(PathBuf, u64), // path and file size
}

#[derive(Debug, Clone)]
pub enum Message {
    Next,
    Previous,
    Enter,
    Show,
    Close,
}

#[derive(Debug, Clone)]
pub enum UpdateResult {
    None,
    Selected(PathBuf),
}

pub struct FileSelector {
    pub current_dir: PathBuf,
    pub dir_entries: Vec<DirEntry>,
    pub state: ListState,
    pub is_open: bool,
}

impl FileSelector {
    pub fn new() -> Self {
        Self {
            current_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            dir_entries: Vec::new(),
            state: ListState::default(),
            is_open: false,
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
            KeyCode::Enter => Some(Message::Enter),
            _ => None,
        }
    }

    pub fn update(&mut self, msg: Message) -> UpdateResult {
        match msg {
            Message::Next => {
                self.next_entry();
                UpdateResult::None
            }
            Message::Previous => {
                self.previous_entry();
                UpdateResult::None
            }
            Message::Enter => {
                if let Some(path) = self.enter_selected_entry() {
                    UpdateResult::Selected(path)
                } else {
                    UpdateResult::None
                }
            }
            Message::Show => {
                self.show();
                self.is_open = true;
                UpdateResult::None
            }
            Message::Close => {
                self.close();
                UpdateResult::None
            }
        }
    }

    pub fn view(&mut self, frame: &mut Frame, area: Rect) {
        let colors = get_colors();

        let current_dir_display = self
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
                    .fg(colors.text_primary)
                    .add_modifier(Modifier::BOLD),
            )]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(colors.border_active))
            .style(Style::default().bg(colors.bg_dialog));

        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Min(1),    // File list
                Constraint::Length(1), // Instructions
            ])
            .split(area);

        // Directory and file list
        if self.dir_entries.is_empty() {
            let no_files_widget = Paragraph::new("Directory is empty or cannot be accessed")
                .style(Style::default().fg(colors.text_secondary))
                .alignment(ratatui::layout::Alignment::Center);
            frame.render_widget(no_files_widget, chunks[0]);
        } else {
            let items: Vec<ListItem> = self
                .dir_entries
                .iter()
                .map(|entry| match entry {
                    DirEntry::Parent => ListItem::new(Line::from(vec![
                        Span::styled("[DIR] ", Style::default().fg(colors.info)),
                        Span::styled(
                            "..",
                            Style::default()
                                .fg(colors.info)
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
                            Span::styled("[DIR] ", Style::default().fg(colors.text_secondary)),
                            Span::styled(dirname, Style::default().fg(colors.text_secondary)),
                        ]))
                    }
                    DirEntry::JsonFile(path, size) => {
                        let filename = path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        let size_str = format_bytes(*size);
                        ListItem::new(Line::from(vec![
                            Span::styled(filename, Style::default().fg(colors.success)),
                            Span::styled(
                                format!(" ({})", size_str),
                                Style::default().fg(colors.success),
                            ),
                        ]))
                    }
                })
                .collect();

            let list = List::new(items)
                .highlight_style(
                    Style::default()
                        .bg(colors.bg_highlight)
                        .fg(colors.text_on_highlight)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ ");

            frame.render_stateful_widget(list, chunks[0], &mut self.state);
        }

        let instructions = Paragraph::new(Line::from(vec![
            Span::styled(
                "↑↓",
                Style::default()
                    .fg(colors.info)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Navigate  ", Style::default().fg(colors.text_primary)),
            Span::styled(
                "Enter",
                Style::default()
                    .fg(colors.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Open/Import  ", Style::default().fg(colors.text_primary)),
            Span::styled(
                "Esc",
                Style::default()
                    .fg(colors.error)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Cancel", Style::default().fg(colors.text_primary)),
        ]))
        .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(instructions, chunks[1]);
    }

    fn close(&mut self) {
        self.is_open = false;
    }

    fn is_hidden_file(path: &std::path::Path) -> bool {
        // Unix/Linux/macOS: Check if the filename starts with a dot (.)
        if let Some(file_name) = path.file_name()
            && file_name.to_string_lossy().starts_with('.')
        {
            return true;
        }

        // Windows: Check file attributes
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if let Ok(metadata) = std::fs::metadata(path) {
                const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
                return (metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN) != 0;
            }
        }

        false
    }

    fn show(&mut self) {
        // Try home directory first, fallback to current directory
        if let Some(home_dir) = dirs::home_dir() {
            self.current_dir = home_dir;
        } else {
            self.current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        }

        self.load_directory_entries();
        if !self.dir_entries.is_empty() {
            self.state.select(Some(0));
        }
    }

    fn load_directory_entries(&mut self) {
        self.dir_entries.clear();

        // Add parent directory entry if not at root
        if self.current_dir.parent().is_some() {
            self.dir_entries.push(DirEntry::Parent);
        }

        if let Ok(entries) = std::fs::read_dir(&self.current_dir) {
            let mut dirs = Vec::new();
            let mut files = Vec::new();

            for entry in entries.flatten() {
                let path = entry.path();

                if Self::is_hidden_file(&path) {
                    continue;
                }

                if path.is_dir() {
                    dirs.push(DirEntry::Directory(path));
                } else if let Some(extension) = path.extension()
                    && extension
                        .to_str()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
                {
                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    files.push(DirEntry::JsonFile(path, size));
                }
            }

            // Sort directories and files separately
            dirs.sort_by_key(|entry| {
                let DirEntry::Directory(path) = entry else {
                    unreachable!()
                };
                path.file_name().map(|name| name.to_os_string())
            });
            files.sort_by_key(|entry| {
                let DirEntry::JsonFile(path, _) = entry else {
                    unreachable!()
                };
                path.file_name().map(|name| name.to_os_string())
            });

            // Add directories first, then files
            self.dir_entries.extend(dirs);
            self.dir_entries.extend(files);
        }
    }

    fn next_entry(&mut self) {
        if self.dir_entries.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.dir_entries.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous_entry(&mut self) {
        if self.dir_entries.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.dir_entries.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn enter_selected_entry(&mut self) -> Option<PathBuf> {
        if let Some(selected_idx) = self.state.selected() {
            if let Some(entry) = self.dir_entries.get(selected_idx) {
                match entry {
                    DirEntry::Parent => {
                        if let Some(parent) = self.current_dir.parent() {
                            self.current_dir = parent.to_path_buf();
                            self.load_directory_entries();
                            self.state.select(Some(0));
                        }
                        None
                    }
                    DirEntry::Directory(path) => {
                        self.current_dir = path.clone();
                        self.load_directory_entries();
                        self.state.select(Some(0));
                        None
                    }
                    DirEntry::JsonFile(path, _) => Some(path.clone()),
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}
