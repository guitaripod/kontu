use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

/// Persisted user configuration, stored as TOML in the platform config dir
/// (`~/.config/kontu/config.toml` on Linux).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Base URL of the kontu Cloudflare Worker.
    pub server_url: String,
    /// Bearer token sent to the Worker's authenticated API.
    pub api_token: String,
    /// Active color theme name.
    pub theme: String,
    /// Telegram bot token for `kontu watch` new-listing alerts (from @BotFather).
    pub telegram_token: String,
    /// Telegram chat id alerts are delivered to (auto-detected on first message).
    pub telegram_chat_id: String,
    /// Public base URL of the kontu web app (where validated listings are
    /// published); Telegram alerts link to `<webapp_url>/kontu/<id>`.
    #[serde(default = "default_webapp_url")]
    pub webapp_url: String,
}

fn default_webapp_url() -> String {
    "https://kontu.guitaripod.workers.dev".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:8787".to_string(),
            api_token: String::new(),
            theme: "default".to_string(),
            telegram_token: String::new(),
            telegram_chat_id: String::new(),
            webapp_url: "https://kontu.guitaripod.workers.dev".to_string(),
        }
    }
}

impl Config {
    fn project_dirs() -> Result<ProjectDirs> {
        ProjectDirs::from("ml", "Kontu", "kontu")
            .context("could not determine a home directory for config")
    }

    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::project_dirs()?.config_dir().join("config.toml"))
    }

    /// Load config, creating a default file on first run. `KONTU_SERVER_URL` and
    /// `KONTU_API_TOKEN` environment variables override the file when set.
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        let mut cfg = if !path.exists() {
            let cfg = Self::default();
            cfg.save()?;
            cfg
        } else {
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?
        };
        cfg.apply_env_overrides();
        Ok(cfg)
    }

    /// Parse the on-disk config WITHOUT applying environment overrides. Use this
    /// before any `save()` so env-only secrets (e.g. `KONTU_API_TOKEN`) are not
    /// persisted into the file.
    pub fn load_raw() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(v) = std::env::var("KONTU_SERVER_URL")
            && !v.is_empty() {
                self.server_url = v;
            }
        if let Ok(v) = std::env::var("KONTU_API_TOKEN")
            && !v.is_empty() {
                self.api_token = v;
            }
        if let Ok(v) = std::env::var("KONTU_TELEGRAM_TOKEN")
            && !v.is_empty() {
                self.telegram_token = v;
            }
        if let Ok(v) = std::env::var("KONTU_TELEGRAM_CHAT_ID")
            && !v.is_empty() {
                self.telegram_chat_id = v;
            }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let text = toml::to_string_pretty(self).context("serializing config")?;
        std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_round_trips_through_toml() {
        let cfg = Config::default();
        let text = toml::to_string_pretty(&cfg).unwrap();
        let parsed: Config = toml::from_str(&text).unwrap();
        assert_eq!(cfg, parsed);
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let parsed: Config = toml::from_str("server_url = \"https://example.com\"").unwrap();
        assert_eq!(parsed.server_url, "https://example.com");
        assert_eq!(parsed.theme, Config::default().theme);
    }
}
