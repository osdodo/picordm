use std::time::Duration;

use anyhow::Result;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
};

use crate::models::Screen;
use crate::screens::{connection_screen, dashboard_screen};
use crate::theme::init_theme_manager;
use crate::widgets::{
    footer,
    settings_dialog::SettingsDialog,
    settings_storage::load_theme,
};

pub struct App {
    pub current_screen: Screen,
    pub connection_screen: connection_screen::ConnectionScreen,
    pub dashboard_screen: dashboard_screen::DashboardScreen,
    pub settings_dialog: SettingsDialog,
}

impl App {
    pub fn new() -> Self {
        let theme = load_theme();
        init_theme_manager(theme);

        Self {
            current_screen: Screen::Connection,
            connection_screen: connection_screen::ConnectionScreen::new(),
            dashboard_screen: dashboard_screen::DashboardScreen::new(),
            settings_dialog: SettingsDialog::new(),
        }
    }

    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|frame| self.view(frame))?;

            if event::poll(Duration::from_millis(100))?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                let should_continue = self.handle_key_events(key, terminal).await?;
                if !should_continue {
                    break;
                }
            }
        }

        Ok(())
    }

    fn view(&mut self, frame: &mut Frame) {
        match self.current_screen {
            Screen::Connection => {
                self.connection_screen.view(frame);
            }
            Screen::Dashboard => {
                self.dashboard_screen.view(frame);
            }
        }

        if self.settings_dialog.is_open {
            self.settings_dialog.view(frame);
        }
    }

    async fn handle_key_events(
        &mut self,
        key: KeyEvent,
        terminal: &mut DefaultTerminal,
    ) -> Result<bool> {
        // Global shortcuts
        match key.code {
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(false);
            }
            KeyCode::F(10) => {
                self.settings_dialog.show();
                return Ok(true);
            }
            _ => {}
        }

        if self.settings_dialog.is_open && self.settings_dialog.handle_key_event(key) {
            return Ok(true);
        }

        match self.current_screen {
            Screen::Connection => {
                if let Some(msg) = self.connection_screen.handle_key_events(key)
                    && let connection_screen::UpdateResult::SwitchScreen(screen, config) =
                        self.connection_screen.update(msg, terminal).await?
                {
                    match self.dashboard_screen.load_data(&config, terminal).await {
                        Err(e) => {
                            self.connection_screen
                                .footer
                                .update(footer::Message::Error(Some(format!(
                                    "Failed to load dashboard: {}",
                                    e
                                ))));
                            return Ok(true);
                        }
                        Ok(_) => {
                            self.current_screen = screen;
                        }
                    }
                }
            }
            Screen::Dashboard => {
                if let Some(msg) = self.dashboard_screen.handle_key_events(key)
                    && let dashboard_screen::UpdateResult::SwitchScreen(screen) =
                        self.dashboard_screen.update(msg, terminal).await?
                {
                    self.current_screen = screen;
                }
            }
        }

        Ok(true)
    }
}
