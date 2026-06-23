use serde::{Deserialize, Serialize};

/// 2026 seed defaults for the cost engine (see `SPEC.md` §2/§3). Mirrors the
/// Worker `cost_defaults` table so the TUI works offline; the Worker can supply
/// verified overrides via `/api/cost-defaults`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CostDefaults {
    pub transfer_tax_kiinteisto: f64,
    pub transfer_tax_osake: f64,
    pub lainhuuto_eur: f64,
    pub kaupanvahvistus_eur: f64,
    pub kaupanvahvistus_econveyance_eur: f64,
    pub kiinnitys_eur: f64,
    pub kuntotarkastus_eur: f64,
    pub euribor_12m: f64,
    pub mortgage_margin: f64,
    pub ltv_max: f64,
    pub ltv_first_home: f64,
    pub loan_term_years: u32,
    pub kvero_building_permanent_min: f64,
    pub kvero_building_permanent_max: f64,
    pub kvero_building_general_min: f64,
    pub kvero_building_general_max: f64,
    pub kvero_land_min: f64,
    pub kvero_land_max: f64,
    pub insurance_eur_yr: f64,
    pub heating_maalampo_eur_yr: f64,
    pub heating_kaukolampo_eur_yr: f64,
    pub heating_ivlp_eur_yr: f64,
    pub heating_oljy_eur_yr: f64,
    pub heating_sahko_eur_yr: f64,
    pub heating_puu_eur_yr: f64,
    pub electricity_eur_yr: f64,
    pub water_municipal_eur_yr: f64,
    pub water_well_eur_yr: f64,
    pub waste_eur_yr: f64,
    pub nuohous_eur_yr: f64,
    pub tiekunta_eur_yr: f64,
    pub broadband_eur_yr: f64,
    pub maintenance_reserve_pct: f64,
    pub discount_rate_real: f64,
    pub general_inflation: f64,
    pub energy_inflation: f64,
    pub resale_real_growth: f64,
    pub seller_commission_pct: f64,
}

impl Default for CostDefaults {
    fn default() -> Self {
        Self {
            transfer_tax_kiinteisto: 0.03,
            transfer_tax_osake: 0.015,
            lainhuuto_eur: 172.0,
            kaupanvahvistus_eur: 143.0,
            kaupanvahvistus_econveyance_eur: 0.0,
            kiinnitys_eur: 47.0,
            kuntotarkastus_eur: 1450.0,
            euribor_12m: 0.02809,
            mortgage_margin: 0.0052,
            ltv_max: 0.90,
            ltv_first_home: 0.95,
            loan_term_years: 25,
            kvero_building_permanent_min: 0.0041,
            kvero_building_permanent_max: 0.0100,
            kvero_building_general_min: 0.0093,
            kvero_building_general_max: 0.0200,
            kvero_land_min: 0.0130,
            kvero_land_max: 0.0200,
            insurance_eur_yr: 450.0,
            heating_maalampo_eur_yr: 900.0,
            heating_kaukolampo_eur_yr: 2200.0,
            heating_ivlp_eur_yr: 1400.0,
            heating_oljy_eur_yr: 3100.0,
            heating_sahko_eur_yr: 4000.0,
            heating_puu_eur_yr: 1200.0,
            electricity_eur_yr: 900.0,
            water_municipal_eur_yr: 850.0,
            water_well_eur_yr: 200.0,
            waste_eur_yr: 300.0,
            nuohous_eur_yr: 110.0,
            tiekunta_eur_yr: 400.0,
            broadband_eur_yr: 500.0,
            maintenance_reserve_pct: 0.015,
            discount_rate_real: 0.03,
            general_inflation: 0.02,
            energy_inflation: 0.04,
            resale_real_growth: 0.0,
            seller_commission_pct: 0.035,
        }
    }
}

impl CostDefaults {
    /// Mid-band kiinteistövero estimate: the building band (permanent residence,
    /// or general/leisure for a mökki/loma property) plus the land component.
    /// Inputs are verotusarvo proxies, well below market value.
    pub fn estimated_kiinteistovero(
        &self,
        building_taxable: f64,
        land_taxable: f64,
        is_leisure: bool,
    ) -> f64 {
        let (bmin, bmax) = if is_leisure {
            (self.kvero_building_general_min, self.kvero_building_general_max)
        } else {
            (self.kvero_building_permanent_min, self.kvero_building_permanent_max)
        };
        building_taxable * (bmin + bmax) / 2.0
            + land_taxable * (self.kvero_land_min + self.kvero_land_max) / 2.0
    }
}
