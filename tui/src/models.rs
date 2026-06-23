//! Shared data shapes mirroring the Worker API / D1 schema (see `SPEC.md` §5/§6).

use serde::{Deserialize, Deserializer, Serialize};

use crate::cost::HeatingType;
use crate::risk::RiskInput;

/// Accept a SQLite-style boolean: JSON `true/false`, integer `0/1`, or null.
fn de_opt_bool<'de, D>(d: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Option::<serde_json::Value>::deserialize(d)?;
    Ok(match v {
        Some(serde_json::Value::Bool(b)) => Some(b),
        Some(serde_json::Value::Number(n)) => Some(n.as_i64().map(|i| i != 0).unwrap_or(false)),
        _ => None,
    })
}

/// One portal listing as served by `/api/listings`. Most fields are optional
/// because source data is sparse and drifts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Listing {
    pub id: i64,
    #[serde(default)]
    pub property_id: Option<i64>,
    pub portal: String,
    pub portal_listing_id: String,
    pub url: String,

    #[serde(default)]
    pub property_type: Option<String>,
    #[serde(default)]
    pub holding_form: Option<String>,
    #[serde(default)]
    pub kiinteistotunnus: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub municipality: Option<String>,
    #[serde(default)]
    pub postal_code: Option<String>,
    #[serde(default)]
    pub district: Option<String>,
    #[serde(default)]
    pub lat: Option<f64>,
    #[serde(default)]
    pub lon: Option<f64>,

    #[serde(default)]
    pub price_eur: Option<i64>,
    #[serde(default)]
    pub debt_free_price_eur: Option<i64>,
    #[serde(default)]
    pub debt_share_eur: Option<i64>,
    #[serde(default)]
    pub price_per_m2: Option<f64>,
    #[serde(default)]
    pub maintenance_charge_eur: Option<i64>,
    #[serde(default)]
    pub financing_charge_eur: Option<i64>,
    #[serde(default)]
    pub ground_rent_eur_yr: Option<i64>,

    #[serde(default)]
    pub living_area_m2: Option<f64>,
    #[serde(default)]
    pub total_area_m2: Option<f64>,
    #[serde(default)]
    pub plot_area_m2: Option<f64>,
    #[serde(default)]
    pub room_count: Option<f64>,
    #[serde(default)]
    pub room_layout: Option<String>,
    #[serde(default)]
    pub floors: Option<f64>,

    #[serde(default)]
    pub year_built: Option<i32>,
    #[serde(default)]
    pub occupancy_year: Option<i32>,
    #[serde(default)]
    pub condition_class: Option<String>,
    #[serde(default)]
    pub inspection_status: Option<String>,
    #[serde(default)]
    pub frame_material: Option<String>,
    #[serde(default)]
    pub facade_material: Option<String>,
    #[serde(default)]
    pub roof_material: Option<String>,
    #[serde(default)]
    pub energy_class: Option<String>,
    #[serde(default)]
    pub e_value: Option<f64>,
    #[serde(default)]
    pub risk_structures: Vec<String>,

    #[serde(default)]
    pub plot_ownership: Option<String>,
    #[serde(default)]
    pub lease_end_year: Option<i32>,
    #[serde(default)]
    pub shore: Option<String>,
    #[serde(default, deserialize_with = "de_opt_bool")]
    pub shore_sauna: Option<bool>,

    #[serde(default)]
    pub heating_type: Option<String>,
    #[serde(default)]
    pub heat_distribution: Option<String>,
    #[serde(default)]
    pub water_supply: Option<String>,
    #[serde(default)]
    pub sewer_system: Option<String>,
    #[serde(default)]
    pub broadband: Option<String>,
    #[serde(default)]
    pub sauna: Option<String>,
    #[serde(default)]
    pub parking: Option<String>,
    #[serde(default)]
    pub road_access: Option<String>,
    #[serde(default)]
    pub intended_use: Option<String>,
    #[serde(default)]
    pub zoning_status: Option<String>,

    #[serde(default = "default_status")]
    pub status: String,
    pub first_seen: i64,
    pub last_seen: i64,

    #[serde(default)]
    pub days_on_market: Option<i64>,
    #[serde(default)]
    pub personal_score: Option<i32>,
    #[serde(default)]
    pub risk_score: Option<u32>,
}

fn default_status() -> String {
    "active".to_string()
}

impl Listing {
    /// €/m², from the stored value or computed from price and living area.
    pub fn effective_ppm2(&self) -> Option<f64> {
        if let Some(v) = self.price_per_m2 {
            return Some(v);
        }
        match (self.price_eur, self.living_area_m2) {
            (Some(p), Some(a)) if a > 0.0 => Some(p as f64 / a),
            _ => None,
        }
    }

    /// Best-effort label for the table/detail header.
    pub fn title(&self) -> String {
        self.address
            .clone()
            .or_else(|| self.municipality.clone())
            .unwrap_or_else(|| format!("{}#{}", self.portal, self.portal_listing_id))
    }

    /// Map the listing's heating string to the cost engine's enum (defaults to
    /// district heating when unknown).
    pub fn heating_enum(&self) -> HeatingType {
        match self.heating_type.as_deref().map(str::to_lowercase) {
            Some(h) if h.contains("maalämpö") || h.contains("maalampo") => HeatingType::Maalampo,
            Some(h) if h.contains("öljy") || h.contains("oljy") => HeatingType::Oljy,
            Some(h) if h.contains("ilma") => HeatingType::IlmaLampopumppu,
            Some(h) if h.contains("puu") || h.contains("pelletti") => HeatingType::Puu,
            Some(h) if h.contains("sähkö") || h.contains("sahko") => HeatingType::Sahko,
            _ => HeatingType::Kaukolampo,
        }
    }

    /// Build a [`RiskInput`] from this listing. `near_water` comes from the
    /// location dossier (or the shore field as a fallback).
    pub fn to_risk_input(&self, near_water: bool) -> RiskInput {
        RiskInput {
            build_year: self.year_built,
            risk_structures: self.risk_structures.clone(),
            heating: self.heating_type.clone(),
            roof_material: self.roof_material.clone(),
            roof_year: None,
            condition_class: self.condition_class.clone(),
            inspection_done: self
                .inspection_status
                .as_deref()
                .map(|s| s.to_lowercase().contains("tehty"))
                .unwrap_or(false),
            sewer_system: self.sewer_system.clone(),
            near_water_or_groundwater: near_water
                || self
                    .shore
                    .as_deref()
                    .map(|s| s.contains("ranta"))
                    .unwrap_or(false),
            pipes_renovated_year: None,
        }
    }
}

/// Columns the list view can sort by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortColumn {
    Price,
    PricePerM2,
    SizeM2,
    YearBuilt,
    DaysOnMarket,
    RiskScore,
    Score,
}

impl SortColumn {
    pub fn as_param(&self) -> &'static str {
        match self {
            SortColumn::Price => "price",
            SortColumn::PricePerM2 => "ppm2",
            SortColumn::SizeM2 => "size",
            SortColumn::YearBuilt => "year",
            SortColumn::DaysOnMarket => "dom",
            SortColumn::RiskScore => "risk",
            SortColumn::Score => "score",
        }
    }
}

/// Exact-parameter filter (the core feature). Serialized to query params for the API.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FilterState {
    pub municipality: Option<String>,
    pub property_type: Option<String>,
    pub holding_form: Option<String>,
    pub price_min: Option<i64>,
    pub price_max: Option<i64>,
    pub m2_min: Option<f64>,
    pub m2_max: Option<f64>,
    pub rooms_min: Option<f64>,
    pub year_min: Option<i32>,
    pub shore: Option<String>,
    pub heating_type: Option<String>,
    pub energy_class_max: Option<String>,
    pub plot_ownership: Option<String>,
    pub max_days_on_market: Option<i64>,
    pub exclude_keywords: Vec<String>,
    pub price_dropped: bool,
    pub text: Option<String>,
}

impl FilterState {
    /// Encode as `(key, value)` query pairs for the listings endpoint. Empty
    /// fields are omitted so the API only filters on what is set.
    pub fn to_query_pairs(&self) -> Vec<(String, String)> {
        let mut q: Vec<(String, String)> = Vec::new();
        let mut s = |k: &str, v: &Option<String>| {
            if let Some(v) = v
                && !v.is_empty() {
                    q.push((k.to_string(), v.clone()));
                }
        };
        s("municipality", &self.municipality);
        s("property_type", &self.property_type);
        s("holding_form", &self.holding_form);
        s("shore", &self.shore);
        s("heating_type", &self.heating_type);
        s("energy_class_max", &self.energy_class_max);
        s("plot_ownership", &self.plot_ownership);
        s("text", &self.text);
        let mut n = |k: &str, v: Option<i64>| {
            if let Some(v) = v {
                q.push((k.to_string(), v.to_string()));
            }
        };
        n("price_min", self.price_min);
        n("price_max", self.price_max);
        n("year_min", self.year_min.map(|y| y as i64));
        n("max_days_on_market", self.max_days_on_market);
        if let Some(v) = self.m2_min {
            q.push(("m2_min".into(), v.to_string()));
        }
        if let Some(v) = self.m2_max {
            q.push(("m2_max".into(), v.to_string()));
        }
        if let Some(v) = self.rooms_min {
            q.push(("rooms_min".into(), v.to_string()));
        }
        if self.price_dropped {
            q.push(("price_dropped".into(), "1".into()));
        }
        for kw in &self.exclude_keywords {
            q.push(("exclude".into(), kw.clone()));
        }
        q
    }
}

/// A page of listings from `/api/listings`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListingsPage {
    #[serde(default)]
    pub listings: Vec<Listing>,
    #[serde(default)]
    pub total: i64,
}

/// A price/status change event from the listing history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListingEvent {
    pub kind: String,
    #[serde(default)]
    pub old_price_eur: Option<i64>,
    #[serde(default)]
    pub new_price_eur: Option<i64>,
    #[serde(default)]
    pub old_value: Option<String>,
    #[serde(default)]
    pub new_value: Option<String>,
    pub observed_at: i64,
}

/// A cached listing photo (served from R2 via the Worker).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Photo {
    pub r2_key: String,
    #[serde(default)]
    pub content_type: Option<String>,
    pub source_url: String,
    #[serde(default)]
    pub position: i64,
}

/// Full detail for one listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListingDetail {
    pub listing: Listing,
    #[serde(default)]
    pub events: Vec<ListingEvent>,
    #[serde(default)]
    pub photos: Vec<Photo>,
    #[serde(default)]
    pub dossier: Option<serde_json::Value>,
    #[serde(default)]
    pub cost_inputs: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_ppm2_computes_when_missing() {
        let mut l = Listing {
            price_eur: Some(200_000),
            living_area_m2: Some(100.0),
            ..Default::default()
        };
        assert_eq!(l.effective_ppm2(), Some(2000.0));
        l.price_per_m2 = Some(1900.0);
        assert_eq!(l.effective_ppm2(), Some(1900.0));
    }

    #[test]
    fn heating_enum_maps_finnish_strings() {
        let heating = |s: Option<&str>| {
            Listing {
                heating_type: s.map(str::to_string),
                ..Default::default()
            }
            .heating_enum()
        };
        assert_eq!(heating(Some("Maalämpö")), HeatingType::Maalampo);
        assert_eq!(heating(Some("Öljylämmitys")), HeatingType::Oljy);
        assert_eq!(heating(None), HeatingType::Kaukolampo);
    }

    #[test]
    fn filter_only_emits_set_fields() {
        let f = FilterState {
            municipality: Some("Outokumpu".into()),
            price_max: Some(120_000),
            shore: Some("oma_ranta".into()),
            exclude_keywords: vec!["vuokratontti".into()],
            price_dropped: true,
            ..Default::default()
        };
        let pairs = f.to_query_pairs();
        assert!(pairs.contains(&("municipality".into(), "Outokumpu".into())));
        assert!(pairs.contains(&("price_max".into(), "120000".into())));
        assert!(pairs.contains(&("shore".into(), "oma_ranta".into())));
        assert!(pairs.contains(&("exclude".into(), "vuokratontti".into())));
        assert!(pairs.contains(&("price_dropped".into(), "1".into())));
        assert!(!pairs.iter().any(|(k, _)| k == "price_min"));
    }
}
