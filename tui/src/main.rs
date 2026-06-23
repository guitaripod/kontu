mod action;
mod api;
mod app;
mod config;
mod cost;
mod format;
mod logging;
mod models;
mod probe;
#[cfg(test)]
mod render_smoke;
mod risk;
mod theme;
mod tui;
mod ui;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let _log_guard = logging::init()?;
    let config = config::Config::load()?;
    let client = api::KontuClient::new(config.server_url.clone(), config.api_token.clone())?;

    if std::env::args().any(|a| a == "--probe") {
        return probe::run(&client).await;
    }

    tracing::info!(server = %config.server_url, "kontu starting");
    let picker = ratatui_image::picker::Picker::from_query_stdio().ok();
    let mut app = app::App::new(client, &config, picker);

    let mut tui = tui::Tui::new()?;
    let result = app.run(&mut tui).await;
    drop(tui);

    if let Err(err) = &result {
        tracing::error!(error = %err, "kontu exited with error");
        eprintln!("kontu: {err}");
    }
    result
}
