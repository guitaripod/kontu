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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:8787".to_string(),
            api_token: String::new(),
            theme: "default".to_string(),
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

    /// Load config, creating a default file on first run.
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            let cfg = Self::default();
            cfg.save()?;
            return Ok(cfg);
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
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
