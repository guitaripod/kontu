use serde::{Deserialize, Serialize};

use crate::country::Country;

/// 2026 seed defaults for the cost engine (see `SPEC.md` §2/§3). Mirrors the
/// Worker `cost_defaults` table so the TUI works offline; the Worker can supply
/// verified overrides via `/api/cost-defaults`. The numbers are per-country —
/// build an instance with [`CostDefaults::for_country`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CostDefaults {
    /// Which Nordic market these seed values describe.
    pub country: Country,
    /// Units of local currency per EUR (FI = 1.0; SE ≈ 11.3; the cost engine is
    /// EUR-internal, so fixed local-currency fees are divided by this).
    pub local_per_eur: f64,
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
            country: Country::Fi,
            local_per_eur: 1.0,
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
    /// Seed defaults for a country, from the verified facts packs in
    /// `docs/expansion/<c>.md`. Uncalibrated markets inherit the Finnish shape
    /// and are gated out by [`Country::cost_calibrated`].
    pub fn for_country(country: Country) -> Self {
        match country {
            Country::Fi => CostDefaults::default(),
            Country::Se => CostDefaults::sweden(),
            Country::No => CostDefaults::norway(),
            Country::Dk => CostDefaults::denmark(),
            Country::Is => CostDefaults::iceland(),
        }
    }

    /// Resolve defaults for a country, preferring the Worker-provided Finnish
    /// overrides for Finland and the built-in calibrated seeds for the rest.
    pub fn resolve(worker_fi: &CostDefaults, country: Country) -> CostDefaults {
        if country == Country::Fi {
            worker_fi.clone()
        } else {
            CostDefaults::for_country(country)
        }
    }

    /// Sweden 2026 seeds (docs/expansion/se.md). Recurring lines are SEK figures
    /// converted to EUR at 11.3 SEK/EUR; rates/structure live in the Sweden
    /// [`super::jurisdiction::Sweden`] jurisdiction. Heating reuses the shared
    /// enum: kaukolämpö↔fjärrvärme, maalämpö↔bergvärme, ivlp↔luftvärmepump,
    /// sähkö↔direktel, puu↔pellets, öljy↔olja.
    fn sweden() -> Self {
        Self {
            country: Country::Se,
            local_per_eur: 11.3,
            // Stämpelskatt: 1.5% on freehold (fastighet/tomträtt); bostadsrätt none.
            transfer_tax_kiinteisto: 0.015,
            transfer_tax_osake: 0.0,
            insurance_eur_yr: 560.0,         // villaförsäkring ~6,300 SEK
            heating_kaukolampo_eur_yr: 1650.0, // fjärrvärme ~18,500 SEK
            heating_maalampo_eur_yr: 1150.0,   // bergvärme ~13,000 SEK
            heating_ivlp_eur_yr: 1050.0,       // luftvärmepump ~12,000 SEK
            heating_oljy_eur_yr: 2850.0,       // olja ~32,000 SEK
            heating_sahko_eur_yr: 2100.0,      // direktverkande el ~24,000 SEK
            heating_puu_eur_yr: 1250.0,        // pellets/ved ~14,000 SEK
            electricity_eur_yr: 1300.0,        // non-heat ~10,000 + elnät ~5,000 SEK
            water_municipal_eur_yr: 930.0,     // kommunalt VA ~10,500 SEK
            water_well_eur_yr: 350.0,          // egen brunn + off-grid avlopp upkeep
            waste_eur_yr: 245.0,               // sophämtning ~2,750 SEK
            nuohous_eur_yr: 40.0,              // sotning ~450 SEK (if hearth)
            tiekunta_eur_yr: 155.0,            // samfällighet ~1,750 SEK (if private road)
            broadband_eur_yr: 425.0,           // villafiber ~4,800 SEK
            ..Default::default()
        }
    }

    /// Norway 2026 seeds (docs/expansion/no.md). NOK figures converted at 11.2
    /// NOK/EUR; rates/structure live in [`super::jurisdiction::Norway`]. Heating
    /// reuses the shared enum: kaukolämpö↔fjernvarme, maalämpö↔bergvarme,
    /// ivlp↔luft-vann-varmepumpe, sähkö↔panelovner, puu↔vedfyring, öljy↔oljefyr.
    fn norway() -> Self {
        Self {
            country: Country::No,
            local_per_eur: 11.2,
            // Dokumentavgift: 2.5% on freehold (Selveier); co-op (Andel/Aksje) none.
            transfer_tax_kiinteisto: 0.025,
            transfer_tax_osake: 0.0,
            insurance_eur_yr: 940.0,           // husforsikring ~10,500 NOK
            heating_kaukolampo_eur_yr: 1875.0, // fjernvarme ~21,000 NOK
            heating_maalampo_eur_yr: 1160.0,   // bergvarme ~13,000 NOK
            heating_ivlp_eur_yr: 600.0,        // luft-luft/luft-vann HP (85% of homes)
            heating_oljy_eur_yr: 2860.0,       // oljefyr (banned 2020 — phase-out flag)
            heating_sahko_eur_yr: 2680.0,      // panelovner ~30,000 NOK
            heating_puu_eur_yr: 1030.0,        // vedfyring ~11,500 NOK
            electricity_eur_yr: 760.0,         // non-heat ~8,500 NOK
            water_municipal_eur_yr: 1200.0,    // water+sewer share of kommunale avgifter
            water_well_eur_yr: 180.0,          // egen brønn reserve
            waste_eur_yr: 350.0,               // renovasjon share of kommunale avgifter
            nuohous_eur_yr: 45.0,              // feieavgift ~500 NOK (if chimney)
            tiekunta_eur_yr: 450.0,            // veilag ~5,000 NOK (if private road)
            broadband_eur_yr: 800.0,           // fiber ~9,000 NOK
            ..Default::default()
        }
    }

    /// Denmark 2026 seeds (docs/expansion/dk.md). DKK figures converted at the
    /// 7.46 peg; rates/structure live in [`super::jurisdiction::Denmark`]. The
    /// inspection slot carries the buyer's near-universal boligadvokat fee (the
    /// seller funds the tilstandsrapport, so there is no buyer survey).
    fn denmark() -> Self {
        Self {
            country: Country::Dk,
            local_per_eur: 7.46,
            // Tinglysningsafgift variable 0.6% on a freehold (ejerbolig); andel co-op none.
            transfer_tax_kiinteisto: 0.006,
            transfer_tax_osake: 0.0,
            kuntotarkastus_eur: 1000.0,        // boligadvokat ~7,500 DKK
            insurance_eur_yr: 670.0,           // husforsikring ~5,000 DKK
            heating_kaukolampo_eur_yr: 2080.0, // fjernvarme ~15,500 DKK
            heating_maalampo_eur_yr: 1100.0,   // jordvarme HP ~8,200 DKK
            heating_ivlp_eur_yr: 1070.0,       // luft-til-vand HP ~8,000 DKK
            heating_oljy_eur_yr: 3690.0,       // oliefyr ~27,500 DKK (phase-out)
            heating_sahko_eur_yr: 1880.0,      // elvarme ~14,000 DKK
            heating_puu_eur_yr: 2010.0,        // pillefyr ~15,000 DKK
            electricity_eur_yr: 940.0,         // ~7,000 DKK (2026 elafgift cut)
            water_municipal_eur_yr: 1470.0,    // vand+kloak ~11,000 DKK
            water_well_eur_yr: 200.0,
            waste_eur_yr: 600.0,               // affald ~4,500 DKK
            nuohous_eur_yr: 150.0,
            tiekunta_eur_yr: 200.0,
            broadband_eur_yr: 560.0,           // fibernet ~4,200 DKK
            maintenance_reserve_pct: 0.012,
            ..Default::default()
        }
    }

    /// Iceland 2026 seeds (docs/expansion/is.md). ISK figures converted at 144;
    /// rates/structure live in [`super::jurisdiction::Iceland`]. Heating defaults
    /// to cheap geothermal district heat (~90% of homes); there is no holding-form
    /// tax split. Insurance includes the statutory NTÍ catastrophe levy.
    fn iceland() -> Self {
        Self {
            country: Country::Is,
            local_per_eur: 144.0,
            // Stimpilgjald 0.8% — same for any property type (buyer-type drives it).
            transfer_tax_kiinteisto: 0.008,
            transfer_tax_osake: 0.008,
            kuntotarkastus_eur: 760.0,         // ástandsskoðun (optional survey)
            insurance_eur_yr: 750.0,           // fire + NTÍ catastrophe + home
            heating_kaukolampo_eur_yr: 730.0,  // hitaveita geothermal (cheapest in Nordics)
            heating_maalampo_eur_yr: 1250.0,   // varmadæla heat pump
            heating_ivlp_eur_yr: 1250.0,
            heating_oljy_eur_yr: 3400.0,       // olíukynding (off-grid only)
            heating_sahko_eur_yr: 2400.0,      // rafhitun (Westfjords/East)
            heating_puu_eur_yr: 1300.0,
            electricity_eur_yr: 720.0,         // non-heat ~108,000 ISK
            water_municipal_eur_yr: 1075.0,    // vatnsgjald + fráveitugjald
            water_well_eur_yr: 200.0,
            waste_eur_yr: 575.0,               // sorpgjald
            nuohous_eur_yr: 40.0,
            tiekunta_eur_yr: 300.0,
            broadband_eur_yr: 850.0,           // ljósleiðari
            maintenance_reserve_pct: 0.015,
            ..Default::default()
        }
    }

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
