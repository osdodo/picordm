use anyhow::Result;

mod app;
mod models;
mod screens;
mod service;
mod widgets;

use crate::app::App;

#[tokio::main]
async fn main() -> Result<()> {
    let mut terminal = ratatui::init();
    let result = App::new().run(&mut terminal).await;
    ratatui::restore();

    result
}
