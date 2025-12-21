mod app;
mod config;
mod connection;
mod file_selector;
mod handler;
mod impex;
mod service;
mod storage;
mod ui;

use anyhow::Result;

use crate::app::App;

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
    let result = App::new().run(&mut terminal).await;
    ratatui::restore();

    result
}
