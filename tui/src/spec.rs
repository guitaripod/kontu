//! The user's house-hunting spec: the criteria an agent elicits once and reuses.
//! Stored as TOML at `~/.config/kontu/spec.toml`. Drives `kontu match`.

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// How much a soft criterion matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Pref {
    /// Don't care.
    #[default]
    Any,
    /// Nice to have — rewarded but not required.
    Plus,
    /// Must have — listings without it are filtered out (or heavily penalized).
    Required,
    /// Penalize listings that have it.
    Avoid,
}

impl Pref {
    pub fn parse(s: &str) -> Pref {
        match s.to_lowercase().as_str() {
            "required" | "req" | "must" => Pref::Required,
            "plus" | "nice" | "+" => Pref::Plus,
            "avoid" | "no" | "-" => Pref::Avoid,
            _ => Pref::Any,
        }
    }
}

/// Relative weights for the match score (normalized at scoring time).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Weights {
    pub tco: f64,
    pub shore: f64,
    pub privacy: f64,
    pub ev: f64,
    pub fiber: f64,
    pub infra: f64,
    pub winter: f64,
    pub risk: f64,
}

impl Default for Weights {
    fn default() -> Self {
        Self {
            tco: 0.40,
            shore: 0.20,
            privacy: 0.12,
            winter: 0.10,
            infra: 0.08,
            ev: 0.07,
            risk: 0.08,
            fiber: 0.05,
        }
    }
}

/// The house-hunting spec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Spec {
    pub price_max: Option<i64>,
    pub price_min: Option<i64>,
    /// Municipalities to search; empty = anywhere in Finland.
    pub municipalities: Vec<String>,
    /// Property types of interest, e.g. omakotitalo, mökki.
    pub property_types: Vec<String>,
    /// Lakehouse preference.
    pub shore: Pref,
    pub min_plot_m2: Option<f64>,
    pub min_m2: Option<f64>,
    pub min_rooms: Option<f64>,
    pub year_min: Option<i32>,
    /// Prefer an owned plot (penalize vuokratontti — it adds recurring cost).
    pub owned_plot: bool,
    /// Require working everyday infrastructure (water, sewer, electricity, road).
    pub require_infra: bool,
    /// Ability to charge an electric car.
    pub ev_charging: Pref,
    /// Fibre internet.
    pub fiber: Pref,
    /// Not direct neighbours (privacy / rural).
    pub privacy: Pref,
    /// Year-round liveable (talviasuttava) rather than a summer-only mökki.
    /// `Required` hard-drops listings that are clearly summer-only.
    pub winterized: Pref,
    /// Drive the ranking toward the lowest total cost of ownership.
    pub minimize_tco: bool,
    pub max_dom: Option<i64>,
    /// Cost-model horizon in years.
    pub horizon_years: u32,
    /// Exclude listings whose text matches any of these keywords.
    pub exclude: Vec<String>,
    /// Free-text notes (intent the structured fields can't capture).
    pub notes: String,
    pub weights: Weights,
}

impl Default for Spec {
    fn default() -> Self {
        Self {
            price_max: None,
            price_min: None,
            municipalities: Vec::new(),
            property_types: Vec::new(),
            shore: Pref::Any,
            min_plot_m2: None,
            min_m2: None,
            min_rooms: None,
            year_min: None,
            owned_plot: false,
            require_infra: false,
            ev_charging: Pref::Any,
            fiber: Pref::Any,
            privacy: Pref::Any,
            winterized: Pref::Any,
            minimize_tco: false,
            max_dom: None,
            horizon_years: 20,
            exclude: Vec::new(),
            notes: String::new(),
            weights: Weights::default(),
        }
    }
}

impl Spec {
    fn project_dirs() -> Result<ProjectDirs> {
        ProjectDirs::from("ml", "Kontu", "kontu")
            .context("could not determine a home directory for the spec")
    }

    pub fn path() -> Result<PathBuf> {
        Ok(Self::project_dirs()?.config_dir().join("spec.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self).context("serializing spec")?;
        std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))
    }

    /// Whether the spec has any meaningful criteria set.
    pub fn is_empty(&self) -> bool {
        *self == Spec::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_toml() {
        let s = Spec {
            price_max: Some(100_000),
            municipalities: vec!["Outokumpu".into()],
            property_types: vec!["omakotitalo".into(), "mökki".into()],
            shore: Pref::Required,
            ev_charging: Pref::Plus,
            fiber: Pref::Plus,
            minimize_tco: true,
            ..Default::default()
        };
        let text = toml::to_string_pretty(&s).unwrap();
        let parsed: Spec = toml::from_str(&text).unwrap();
        assert_eq!(s, parsed);
    }

    #[test]
    fn pref_parses() {
        assert_eq!(Pref::parse("required"), Pref::Required);
        assert_eq!(Pref::parse("plus"), Pref::Plus);
        assert_eq!(Pref::parse("avoid"), Pref::Avoid);
        assert_eq!(Pref::parse("whatever"), Pref::Any);
    }
}
