use ratatui::widgets::ListState;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum DirEntry {
    Parent,
    Directory(PathBuf),
    JsonFile(PathBuf, u64), // path and file size
}

pub struct FileSelector {
    pub current_dir: PathBuf,
    pub dir_entries: Vec<DirEntry>,
    pub state: ListState,
}

impl FileSelector {
    pub fn new() -> Self {
        Self {
            current_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            dir_entries: Vec::new(),
            state: ListState::default(),
        }
    }

    fn is_hidden_file(path: &std::path::Path) -> bool {
        // Unix/Linux/macOS: Check if the filename starts with a dot (.)
        if let Some(file_name) = path.file_name() {
            if file_name.to_string_lossy().starts_with('.') {
                return true;
            }
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

    pub fn show(&mut self) {
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

    pub fn load_directory_entries(&mut self) {
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
                } else if let Some(extension) = path.extension() {
                    if extension
                        .to_str()
                        .map_or(false, |ext| ext.eq_ignore_ascii_case("json"))
                    {
                        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                        files.push(DirEntry::JsonFile(path, size));
                    }
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

    pub fn next_entry(&mut self) {
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

    pub fn previous_entry(&mut self) {
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

    pub fn enter_selected_entry(&mut self) -> Option<PathBuf> {
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

impl Default for FileSelector {
    fn default() -> Self {
        Self::new()
    }
}
