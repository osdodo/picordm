mod app;
mod config;
mod connection;
mod handler;
mod service;
mod storage;
mod ui;

use std::time::Duration;

use anyhow::Result;
use ratatui::crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event};
use ratatui::crossterm::execute;

use crate::app::App;
use crate::handler::handle_key_event;

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(debug_assertions)]
    {
        simplelog::WriteLogger::init(
            simplelog::LevelFilter::Debug,
            simplelog::Config::default(),
            std::fs::File::create("/tmp/picordm.log")?,
        )?;
    }

    let mut terminal = ratatui::init();
    execute!(std::io::stdout(), EnableMouseCapture)?;

    let result = run(&mut terminal, App::new()).await;

    execute!(std::io::stdout(), DisableMouseCapture)?;
    ratatui::restore();

    result
}

async fn run(terminal: &mut ratatui::DefaultTerminal, mut app: App<'_>) -> Result<()> {
    loop {
        if app.should_execute_connection() {
            app.connect_to_selected().await?;
        }

        if app.should_execute_dashboard_data() {
            app.load_dashboard_data().await?;
        }

        terminal.draw(|f| ui::draw(f, &mut app))?;

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if handle_key_event(terminal, &mut app, key).await? {
                        return Ok(());
                    }
                }
                Event::Mouse(mouse) => {
                    handler::handle_mouse_event(terminal, &mut app, mouse).await?;
                }
                _ => {}
            }
        } else {
            if app.is_connecting
                || app.is_loading_server_info
                || app.is_loading_value
                || app.is_loading_keys
            {
                app.tick_loading();
            }
        }
    }
}
