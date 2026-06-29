mod api;
mod card;
mod cli;
mod config;
mod cost;
mod country;
mod format;
mod ingest;
mod matching;
mod models;
mod risk;
mod spec;
mod telegram;
mod watch;

use anyhow::Result;
use clap::{CommandFactory, Parser};

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
        None => {
            cli::Cli::command().print_help()?;
            println!();
            Ok(())
        }
    }
}
