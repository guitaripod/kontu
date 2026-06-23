use serde::{Deserialize, Serialize};

/// 2026 seed defaults for the cost engine (see `SPEC.md` §2/§3). Mirrors the
/// Worker `cost_defaults` table so the TUI works offline; the Worker can supply
/// verified overrides via `/api/cost-defaults`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    pub kvero_land_min: f64,
    pub kvero_land_max: f64,
    pub insurance_eur_yr: f64,
    pub heating_maalampo_eur_yr: f64,
    pub heating_kaukolampo_eur_yr: f64,
    pub heating_ilmalampopumppu_eur_yr: f64,
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
            kvero_land_min: 0.0130,
            kvero_land_max: 0.0200,
            insurance_eur_yr: 450.0,
            heating_maalampo_eur_yr: 900.0,
            heating_kaukolampo_eur_yr: 2200.0,
            heating_ilmalampopumppu_eur_yr: 1400.0,
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
    /// Default all-in mortgage rate = 12-mo Euribor + average margin.
    pub fn default_interest_rate(&self) -> f64 {
        self.euribor_12m + self.mortgage_margin
    }

    /// Mid-band kiinteistövero estimate applied to a building rebuild value.
    pub fn estimated_kiinteistovero(&self, building_value: f64) -> f64 {
        building_value * (self.kvero_building_permanent_min + self.kvero_building_permanent_max) / 2.0
    }
}
