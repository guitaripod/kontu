use serde::{Deserialize, Serialize};

use super::defaults::CostDefaults;
use super::{ModelInputs, PropertyInputs};

/// Primary heating system — the largest running-cost driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeatingType {
    Kaukolampo,
    Maalampo,
    Oljy,
    Sahko,
    Puu,
    /// Air-to-water heat pump (ilmavesilämpöpumppu). A bare air-to-air ILP is
    /// not a primary system and maps to `Sahko` instead.
    Ivlp,
}

/// Water/sewer arrangement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaterSupply {
    Municipal,
    Well,
}

impl HeatingType {
    pub fn annual_cost(&self, d: &CostDefaults) -> f64 {
        match self {
            HeatingType::Kaukolampo => d.heating_kaukolampo_eur_yr,
            HeatingType::Maalampo => d.heating_maalampo_eur_yr,
            HeatingType::Oljy => d.heating_oljy_eur_yr,
            HeatingType::Sahko => d.heating_sahko_eur_yr,
            HeatingType::Puu => d.heating_puu_eur_yr,
            HeatingType::Ivlp => d.heating_ivlp_eur_yr,
        }
    }

    /// Whether household electricity is already counted inside the heating line.
    pub fn electricity_included(&self) -> bool {
        matches!(self, HeatingType::Sahko)
    }
}

impl WaterSupply {
    pub fn annual_cost(&self, d: &CostDefaults) -> f64 {
        match self {
            WaterSupply::Municipal => d.water_municipal_eur_yr,
            WaterSupply::Well => d.water_well_eur_yr,
        }
    }
}

/// A recurring cost line with its starting amount and its own nominal escalation.
#[derive(Debug, Clone, PartialEq)]
pub struct RecurringLine {
    pub name: &'static str,
    pub annual0: f64,
    pub inflation: f64,
}

/// Build the recurring cost lines from property inputs and defaults. `building_value`
/// is the resolved building rebuild value (drives the maintenance reserve and the
/// kiinteistövero estimate).
pub fn recurring_lines(
    p: &PropertyInputs,
    m: &ModelInputs,
    d: &CostDefaults,
    building_value: f64,
    kiinteistovero: f64,
) -> Vec<RecurringLine> {
    let g = m.general_inflation;
    let e = m.energy_inflation;

    let mut lines = vec![
        RecurringLine { name: "kiinteistovero", annual0: kiinteistovero, inflation: g },
        RecurringLine { name: "insurance", annual0: p.insurance_eur_yr.unwrap_or(d.insurance_eur_yr), inflation: g },
        RecurringLine { name: "heating", annual0: p.heating.annual_cost(d), inflation: e },
        RecurringLine { name: "water", annual0: p.water.annual_cost(d), inflation: e },
        RecurringLine { name: "waste", annual0: d.waste_eur_yr, inflation: g },
        RecurringLine { name: "broadband", annual0: d.broadband_eur_yr, inflation: g },
        RecurringLine { name: "maintenance_reserve", annual0: building_value * d.maintenance_reserve_pct, inflation: g },
    ];
    if !p.heating.electricity_included() {
        lines.push(RecurringLine {
            name: "electricity",
            annual0: p.electricity_eur_yr.unwrap_or(d.electricity_eur_yr),
            inflation: e,
        });
    }
    if p.fireplace {
        lines.push(RecurringLine { name: "nuohous", annual0: d.nuohous_eur_yr, inflation: g });
    }
    if p.private_road {
        lines.push(RecurringLine { name: "tiekunta", annual0: d.tiekunta_eur_yr, inflation: g });
    }
    if p.ground_rent_eur_yr > 0.0 {
        lines.push(RecurringLine { name: "ground_rent", annual0: p.ground_rent_eur_yr, inflation: g });
    }
    if p.vastike_eur_mo > 0.0 {
        lines.push(RecurringLine { name: "vastike", annual0: p.vastike_eur_mo * 12.0, inflation: g });
    }
    lines
}
