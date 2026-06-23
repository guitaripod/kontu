use std::path::PathBuf;

use anyhow::Result;
use directories::ProjectDirs;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

/// Directory where rotated log files are written
/// (`~/.local/state/kontu/` on Linux, falling back to the local data dir).
pub fn log_dir() -> PathBuf {
    match ProjectDirs::from("ml", "Kontu", "kontu") {
        Some(dirs) => dirs
            .state_dir()
            .unwrap_or_else(|| dirs.data_local_dir())
            .to_path_buf(),
        None => PathBuf::from("."),
    }
}

/// Initialize file-based logging.
///
/// A TUI owns the terminal, so diagnostics must never touch stdout/stderr —
/// they would corrupt the rendered UI. Everything is routed to a daily-rotated
/// file instead. The returned [`WorkerGuard`] must be held for the lifetime of
/// the program or buffered lines are dropped on exit.
pub fn init() -> Result<WorkerGuard> {
    let dir = log_dir();
    std::fs::create_dir_all(&dir).ok();
    let appender = tracing_appender::rolling::daily(&dir, "kontu.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter =
        EnvFilter::try_from_env("KONTU_LOG").unwrap_or_else(|_| EnvFilter::new("info,kontu=debug"));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_ansi(false).with_writer(writer))
        .init();
    Ok(guard)
}
