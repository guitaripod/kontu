mod action;
mod api;
mod app;
mod cli;
mod config;
mod cost;
mod format;
mod ingest;
mod logging;
mod matching;
mod models;
#[cfg(test)]
mod render_smoke;
mod risk;
mod spec;
mod telegram;
mod theme;
mod tui;
mod ui;
mod watch;

use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Cli::parse();

    let mut config = config::Config::load()?;
    if let Some(server) = &args.server {
        config.server_url = server.clone();
    }
    if let Some(token) = &args.token {
        config.api_token = token.clone();
    }
    let client = api::KontuClient::new(config.server_url.clone(), config.api_token.clone())?;

    match args.command {
        Some(command) => cli::run(command, &client, args.json).await,
        None => run_tui(client, &config).await,
    }
}

async fn run_tui(client: api::KontuClient, config: &config::Config) -> Result<()> {
    let _log_guard = logging::init()?;
    tracing::info!(server = %config.server_url, "kontu starting");
    let picker = ratatui_image::picker::Picker::from_query_stdio().ok();
    let mut app = app::App::new(client, config, picker);

    let mut tui = tui::Tui::new()?;
    let result = app.run(&mut tui).await;
    drop(tui);

    if let Err(err) = &result {
        tracing::error!(error = %err, "kontu exited with error");
        eprintln!("kontu: {err}");
    }
    result
}
