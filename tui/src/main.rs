#![allow(dead_code)]

mod action;
mod api;
mod app;
mod config;
mod cost;
mod format;
mod logging;
mod models;
mod risk;
mod theme;
mod tui;
mod ui;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let _log_guard = logging::init()?;
    let config = config::Config::load()?;
    tracing::info!(server = %config.server_url, "kontu starting");

    let client = api::KontuClient::new(config.server_url.clone(), config.api_token.clone())?;
    let mut app = app::App::new(client, &config);

    let mut tui = tui::Tui::new()?;
    let result = app.run(&mut tui).await;
    drop(tui);

    if let Err(err) = &result {
        tracing::error!(error = %err, "kontu exited with error");
        eprintln!("kontu: {err}");
    }
    result
}
