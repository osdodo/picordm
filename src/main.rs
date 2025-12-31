use anyhow::Result;

mod app;
mod constants;
mod models;
mod screens;
mod service;
mod storage;
mod theme;
mod widgets;

use crate::app::App;

#[tokio::main]
async fn main() -> Result<()> {
    let mut terminal = ratatui::init();
    let result = App::new().run(&mut terminal).await;
    ratatui::restore();
    result
}
