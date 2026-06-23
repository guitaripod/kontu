mod config;
mod logging;

fn main() -> anyhow::Result<()> {
    let _log_guard = logging::init()?;
    let cfg = config::Config::load()?;
    tracing::info!(server = %cfg.server_url, "kontu starting");
    eprintln!(
        "kontu — config at {}",
        config::Config::config_path()?.display()
    );
    Ok(())
}
