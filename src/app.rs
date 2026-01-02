use std::time::Duration;

use anyhow::Result;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
};

use crate::models::Screen;
use crate::screens::{connection_screen, dashboard_screen};
use crate::theme::init_theme_manager;
use crate::widgets::{settings_dialog::SettingsDialog, settings_storage::load_settings};

pub struct App {
    pub current_screen: Screen,
    pub connection_screen: connection_screen::ConnectionScreen,
    pub dashboard_screen: dashboard_screen::DashboardScreen,
    pub settings_dialog: SettingsDialog,
}

impl App {
    pub fn new() -> Self {
        let settings = load_settings();
        init_theme_manager(settings.theme, settings.enable_bg_transparent);

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
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.settings_dialog.show();
                return Ok(true);
            }
            _ => {}
        }

        match self.current_screen {
            Screen::Connection => {
                if let Some(msg) = self.connection_screen.handle_key_events(key)
                    && let connection_screen::UpdateResult::SwitchScreen(screen, config) =
                        self.connection_screen.update(msg, terminal).await?
                {
                    self.current_screen = screen;

                    let _ = self
                        .dashboard_screen
                        .update(dashboard_screen::Message::LoadData(*config), terminal)
                        .await;
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

        if self.settings_dialog.is_open {
            self.settings_dialog.handle_key_event(key);
        }

        Ok(true)
    }
}
