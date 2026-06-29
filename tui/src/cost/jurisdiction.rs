//! Country-symmetric acquisition-cost and property-tax rules. kontu is the
//! abstraction; each Nordic country implements [`Jurisdiction`] with its own
//! transfer/stamp tax, registration mechanics and annual property-tax formula.
//! The TCO engine in [`super`] is country-agnostic — it asks the registry for
//! the right jurisdiction and never branches on a country itself.

use serde::{Deserialize, Serialize};

use super::defaults::CostDefaults;
use crate::country::Country;

/// One named registration / conveyancing fee, itemized so each country can show
/// its own line items (FI lainhuuto/kaupanvahvistus/kiinnitys, SE lagfart/pantbrev,
/// NO tinglysingsgebyr, DK tinglysningsafgift, …) instead of a Finnish-shaped total.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeeLine {
    /// Stable machine key (e.g. `"lainhuuto"`).
    pub key: String,
    /// Human label for the breakdown view.
    pub label: String,
    pub amount: f64,
}

impl FeeLine {
    fn new(key: &str, label: &str, amount: f64) -> Self {
        FeeLine { key: key.to_string(), label: label.to_string(), amount }
    }
}

/// The per-country tax & registration rules behind a purchase. The one axis the
/// transfer tax cares about across the Nordics is whether the buyer acquires a
/// **share** in a housing cooperative (FI asunto-osake, SE bostadsrätt, NO
/// borettslag, DK andelsbolig — typically no stamp duty) versus **freehold**
/// real property (the stamp/transfer tax applies).
pub trait Jurisdiction {
    fn country(&self) -> Country;

    /// Property transfer / stamp / title tax on the debt-free price.
    fn transfer_tax(&self, d: &CostDefaults, debt_free_price: f64, is_share: bool) -> f64;

    /// Itemized registration / conveyancing fees at t=0.
    fn registration_fees(
        &self,
        d: &CostDefaults,
        is_share: bool,
        e_conveyance: bool,
        mortgage_deeds: u32,
    ) -> Vec<FeeLine>;

    /// Estimated annual property tax. Each jurisdiction derives its own taxable
    /// base: Finland from the building/land proxies, Sweden from the market
    /// price (taxeringsvärde ≈ 75% of price, capped). `price_eur` is the market
    /// value; `building_taxable`/`land_taxable` are the Finnish-style proxies the
    /// caller already computes (ignored by jurisdictions that don't use them).
    fn annual_property_tax(
        &self,
        d: &CostDefaults,
        price_eur: f64,
        building_taxable: f64,
        land_taxable: f64,
        is_leisure: bool,
    ) -> f64;
}

/// Resolve the jurisdiction for a country.
///
/// Only Finland is calibrated today; Sweden, Norway, Denmark and Iceland get
/// their verified rules in the per-country build (see `docs/expansion/<c>.md`
/// and task #3). Until then they fall back to the Finnish rules, and
/// [`Country::cost_calibrated`] gates them out of the cost commands so no
/// uncalibrated figure is ever shown.
pub fn jurisdiction(country: Country) -> &'static dyn Jurisdiction {
    match country {
        Country::Fi => &Finland,
        Country::Se => &Sweden,
        Country::No => &Norway,
        Country::Dk => &Denmark,
        Country::Is => &Iceland,
    }
}

/// Finland: varainsiirtovero 3.0% (kiinteistö) / 1.5% (asunto-osake) on the
/// debt-free price; lainhuuto + kaupanvahvistus on real property; kiinnitys per
/// mortgage deed; kiinteistövero from the building/land bands.
pub struct Finland;

impl Jurisdiction for Finland {
    fn country(&self) -> Country {
        Country::Fi
    }

    fn transfer_tax(&self, d: &CostDefaults, debt_free_price: f64, is_share: bool) -> f64 {
        let rate = if is_share { d.transfer_tax_osake } else { d.transfer_tax_kiinteisto };
        debt_free_price * rate
    }

    fn registration_fees(
        &self,
        d: &CostDefaults,
        is_share: bool,
        e_conveyance: bool,
        mortgage_deeds: u32,
    ) -> Vec<FeeLine> {
        let mut fees = Vec::new();
        if !is_share {
            fees.push(FeeLine::new("lainhuuto", "Lainhuuto (title registration)", d.lainhuuto_eur));
            let kv = if e_conveyance {
                d.kaupanvahvistus_econveyance_eur
            } else {
                d.kaupanvahvistus_eur
            };
            fees.push(FeeLine::new("kaupanvahvistus", "Kaupanvahvistus (deed)", kv));
        }
        if mortgage_deeds > 0 {
            fees.push(FeeLine::new(
                "kiinnitys",
                "Kiinnitys (mortgage deed)",
                d.kiinnitys_eur * mortgage_deeds as f64,
            ));
        }
        fees
    }

    fn annual_property_tax(
        &self,
        d: &CostDefaults,
        _price_eur: f64,
        building_taxable: f64,
        land_taxable: f64,
        is_leisure: bool,
    ) -> f64 {
        d.estimated_kiinteistovero(building_taxable, land_taxable, is_leisure)
    }
}

/// Sweden (docs/expansion/se.md): stämpelskatt 1.5% on freehold (fastighet /
/// tomträtt), nothing on a bostadsrätt share; a fixed expeditionsavgift to
/// Lantmäteriet; the annual home levy is the capped kommunal fastighetsavgift,
/// min(0.75% × taxeringsvärde, cap), where taxeringsvärde ≈ 75% of price.
pub struct Sweden;

impl Sweden {
    /// Expeditionsavgift (lagfart registration), 825 SEK fixed.
    const EXPEDITION_SEK: f64 = 825.0;
    /// Pantbrev issuance, 375 SEK per new mortgage deed (the 2% is on borrowed
    /// money, which a cash buyer has none of).
    const PANTBREV_SEK: f64 = 375.0;
    /// Kommunal fastighetsavgift: 0.75% of taxeringsvärde, capped (2026).
    const FASTIGHETSAVGIFT_RATE: f64 = 0.0075;
    const FASTIGHETSAVGIFT_CAP_SEK: f64 = 10_425.0;
    /// Taxeringsvärde is assessed at roughly 75% of market value.
    const ASSESSED_FRACTION: f64 = 0.75;
}

impl Jurisdiction for Sweden {
    fn country(&self) -> Country {
        Country::Se
    }

    fn transfer_tax(&self, d: &CostDefaults, debt_free_price: f64, is_share: bool) -> f64 {
        // A bostadsrätt buys co-op shares, not real property — no stamp duty.
        if is_share {
            0.0
        } else {
            debt_free_price * d.transfer_tax_kiinteisto
        }
    }

    fn registration_fees(
        &self,
        d: &CostDefaults,
        is_share: bool,
        _e_conveyance: bool,
        mortgage_deeds: u32,
    ) -> Vec<FeeLine> {
        let mut fees = Vec::new();
        if !is_share {
            fees.push(FeeLine::new(
                "expeditionsavgift",
                "Expeditionsavgift (lagfart)",
                Sweden::EXPEDITION_SEK / d.local_per_eur,
            ));
        }
        if mortgage_deeds > 0 {
            fees.push(FeeLine::new(
                "pantbrev",
                "Pantbrev (mortgage deed)",
                Sweden::PANTBREV_SEK / d.local_per_eur * mortgage_deeds as f64,
            ));
        }
        fees
    }

    fn annual_property_tax(
        &self,
        d: &CostDefaults,
        price_eur: f64,
        _building_taxable: f64,
        _land_taxable: f64,
        _is_leisure: bool,
    ) -> f64 {
        // Permanent and holiday småhus are taxed identically. (The 15-year
        // exemption for värdeår ≥ 2012 is applied once year_built is plumbed
        // through PropertyInputs — TODO(nordic).)
        let taxeringsvarde = Sweden::ASSESSED_FRACTION * price_eur;
        let cap_eur = Sweden::FASTIGHETSAVGIFT_CAP_SEK / d.local_per_eur;
        (Sweden::FASTIGHETSAVGIFT_RATE * taxeringsvarde).min(cap_eur)
    }
}

/// Norway (docs/expansion/no.md): dokumentavgift 2.5% on freehold (Selveier),
/// nothing on a borettslag/aksje co-op; a flat tinglysingsgebyr per document;
/// eiendomsskatt is municipal and optional (no national property tax), estimated
/// here at the national average where levied, on 70% of value.
pub struct Norway;

impl Norway {
    /// Tinglysingsgebyr — flat registration fee per document (deed or pant).
    const TINGLYSING_NOK: f64 = 545.0;
    /// Eiendomsskatt: mandatory 30% reduction → base is 70% of value.
    const EIENDOMSSKATT_FACTOR: f64 = 0.70;
    /// Average rate actually applied where levied (SSB 2026), in per-mille.
    const EIENDOMSSKATT_RATE: f64 = 0.0033;
}

impl Jurisdiction for Norway {
    fn country(&self) -> Country {
        Country::No
    }

    fn transfer_tax(&self, d: &CostDefaults, debt_free_price: f64, is_share: bool) -> f64 {
        // A borettslag/aksje co-op transfers a share, not a deed — no dokumentavgift.
        if is_share {
            0.0
        } else {
            debt_free_price * d.transfer_tax_kiinteisto
        }
    }

    fn registration_fees(
        &self,
        d: &CostDefaults,
        _is_share: bool,
        _e_conveyance: bool,
        mortgage_deeds: u32,
    ) -> Vec<FeeLine> {
        // The flat tinglysingsgebyr is owed on every transfer (deed or co-op share),
        // even when no dokumentavgift is due.
        let fee = Norway::TINGLYSING_NOK / d.local_per_eur;
        let mut fees = vec![FeeLine::new("tinglysingsgebyr", "Tinglysingsgebyr (registration)", fee)];
        if mortgage_deeds > 0 {
            fees.push(FeeLine::new(
                "pantedokument",
                "Pantedokument (mortgage deed)",
                fee * mortgage_deeds as f64,
            ));
        }
        fees
    }

    fn annual_property_tax(
        &self,
        _d: &CostDefaults,
        price_eur: f64,
        _building_taxable: f64,
        _land_taxable: f64,
        _is_leisure: bool,
    ) -> f64 {
        // Eiendomsskatt is municipal and optional (28 of 357 municipalities levy
        // none). With no municipality on hand, estimate the common case: the
        // average rate where levied, on 70% of value, no bunnfradrag. A later
        // geo enrichment can zero this for non-levying municipalities.
        price_eur * Norway::EIENDOMSSKATT_FACTOR * Norway::EIENDOMSSKATT_RATE
    }
}

/// Denmark (docs/expansion/dk.md): tinglysningsafgift 0.6% + fixed fee on a
/// freehold (ejerbolig), nothing on an andelsbolig co-op share; two annual
/// property taxes — ejendomsværdiskat on the dwelling value and grundskyld on
/// the land — both on 80% of the assessed value.
pub struct Denmark;

impl Denmark {
    /// Fixed component of the deed-registration duty.
    const TINGLYSNING_FIXED_DKK: f64 = 1850.0;
    /// Both property taxes use a base of 80% of the public assessment.
    const ASSESSED_FRACTION: f64 = 0.80;
    /// Ejendomsværdiskat: 0.51% up to the progression threshold, 1.4% above.
    const EVS_RATE_LOW: f64 = 0.0051;
    const EVS_RATE_HIGH: f64 = 0.014;
    /// 2026 millionærknæk on the post-deduction base.
    const EVS_THRESHOLD_DKK: f64 = 9_007_000.0;
    /// Grundskyld: national-average municipal rate on the land base.
    const GRUNDSKYLD_RATE: f64 = 0.0074;
    /// Land as a share of a detached house's value (proxy when not itemized).
    const LAND_FRACTION: f64 = 0.35;
}

impl Jurisdiction for Denmark {
    fn country(&self) -> Country {
        Country::Dk
    }

    fn transfer_tax(&self, d: &CostDefaults, debt_free_price: f64, is_share: bool) -> f64 {
        // An andelsbolig is a co-op share, not real property — no deed duty.
        if is_share {
            0.0
        } else {
            debt_free_price * d.transfer_tax_kiinteisto
        }
    }

    fn registration_fees(
        &self,
        d: &CostDefaults,
        is_share: bool,
        _e_conveyance: bool,
        _mortgage_deeds: u32,
    ) -> Vec<FeeLine> {
        let mut fees = Vec::new();
        if !is_share {
            fees.push(FeeLine::new(
                "tinglysningsafgift",
                "Tinglysningsafgift (fixed)",
                Denmark::TINGLYSNING_FIXED_DKK / d.local_per_eur,
            ));
        }
        fees
    }

    fn annual_property_tax(
        &self,
        d: &CostDefaults,
        price_eur: f64,
        _building_taxable: f64,
        _land_taxable: f64,
        _is_leisure: bool,
    ) -> f64 {
        let base = Denmark::ASSESSED_FRACTION * price_eur;
        let threshold = Denmark::EVS_THRESHOLD_DKK / d.local_per_eur;
        let ejendomsvaerdiskat = Denmark::EVS_RATE_LOW * base.min(threshold)
            + Denmark::EVS_RATE_HIGH * (base - threshold).max(0.0);
        let land_base = Denmark::ASSESSED_FRACTION * Denmark::LAND_FRACTION * price_eur;
        let grundskyld = Denmark::GRUNDSKYLD_RATE * land_base;
        ejendomsvaerdiskat + grundskyld
    }
}

/// Iceland (docs/expansion/is.md): stimpilgjald 0.8% with no holding-form split,
/// levied on the assessed `fasteignamat` (≈ 85% of market value); a fixed
/// þinglýsing fee; the municipal fasteignaskattur (property tax) also on the
/// assessed value.
pub struct Iceland;

impl Iceland {
    /// Þinglýsingargjald — fixed registration fee per document.
    const THINGLYSING_ISK: f64 = 3800.0;
    /// Fasteignamat (assessed value) as a fraction of market price — the tax base.
    const ASSESSED_FRACTION: f64 = 0.85;
    /// Fasteignaskattur residential A-rate (rural-weighted; range 0.166–0.625%).
    const PROPERTY_TAX_RATE: f64 = 0.004;
}

impl Jurisdiction for Iceland {
    fn country(&self) -> Country {
        Country::Is
    }

    fn transfer_tax(&self, d: &CostDefaults, debt_free_price: f64, _is_share: bool) -> f64 {
        // No holding-form split; stamp duty is on the assessed fasteignamat.
        debt_free_price * Iceland::ASSESSED_FRACTION * d.transfer_tax_kiinteisto
    }

    fn registration_fees(
        &self,
        d: &CostDefaults,
        _is_share: bool,
        _e_conveyance: bool,
        _mortgage_deeds: u32,
    ) -> Vec<FeeLine> {
        vec![FeeLine::new(
            "thinglysing",
            "Þinglýsingargjald (registration)",
            Iceland::THINGLYSING_ISK / d.local_per_eur,
        )]
    }

    fn annual_property_tax(
        &self,
        _d: &CostDefaults,
        price_eur: f64,
        _building_taxable: f64,
        _land_taxable: f64,
        _is_leisure: bool,
    ) -> f64 {
        // Permanent home and summer house share the A-category rate.
        Iceland::PROPERTY_TAX_RATE * Iceland::ASSESSED_FRACTION * price_eur
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finland_transfer_tax_splits_on_share() {
        let d = CostDefaults::default();
        let j = jurisdiction(Country::Fi);
        assert!((j.transfer_tax(&d, 200_000.0, false) - 6_000.0).abs() < 1e-6);
        assert!((j.transfer_tax(&d, 200_000.0, true) - 3_000.0).abs() < 1e-6);
    }

    #[test]
    fn finland_real_property_has_title_and_deed_fees() {
        let d = CostDefaults::default();
        let fees = jurisdiction(Country::Fi).registration_fees(&d, false, false, 1);
        assert!(fees.iter().any(|f| f.key == "lainhuuto" && f.amount == 172.0));
        assert!(fees.iter().any(|f| f.key == "kaupanvahvistus" && f.amount == 143.0));
        assert!(fees.iter().any(|f| f.key == "kiinnitys" && f.amount == 47.0));
    }

    #[test]
    fn finland_share_skips_real_property_fees_and_cash_skips_deed() {
        let d = CostDefaults::default();
        // Cooperative share: no lainhuuto/kaupanvahvistus.
        let share = jurisdiction(Country::Fi).registration_fees(&d, true, false, 0);
        assert!(!share.iter().any(|f| f.key == "lainhuuto" || f.key == "kaupanvahvistus"));
        // Cash buyer (no mortgage deeds): no kiinnitys.
        assert!(!share.iter().any(|f| f.key == "kiinnitys"));
    }

    #[test]
    fn sweden_stamp_duty_on_freehold_only() {
        let d = CostDefaults::for_country(Country::Se);
        let j = jurisdiction(Country::Se);
        // Freehold (fastighet): 1.5% stämpelskatt.
        assert!((j.transfer_tax(&d, 3_000_000.0, false) - 45_000.0).abs() < 1e-6);
        // Bostadsrätt (co-op share): no stamp duty at all.
        assert_eq!(j.transfer_tax(&d, 3_000_000.0, true), 0.0);
    }

    #[test]
    fn sweden_cash_freehold_pays_only_expeditionsavgift() {
        let d = CostDefaults::for_country(Country::Se);
        let fees = jurisdiction(Country::Se).registration_fees(&d, false, false, 0);
        assert_eq!(fees.len(), 1);
        assert_eq!(fees[0].key, "expeditionsavgift");
        // 825 SEK at 11.3 SEK/EUR ≈ 73 EUR.
        assert!((fees[0].amount - 825.0 / 11.3).abs() < 1e-6);
    }

    #[test]
    fn sweden_property_fee_is_capped() {
        let d = CostDefaults::for_country(Country::Se);
        let j = jurisdiction(Country::Se);
        let cap = Sweden::FASTIGHETSAVGIFT_CAP_SEK / d.local_per_eur;
        // A modest house (the lakeside-cottage profile) is below the cap:
        // 0.75% of 75% of €100k = €562.
        let cheap = j.annual_property_tax(&d, 100_000.0, 0.0, 0.0, false);
        assert!((cheap - 0.0075 * 0.75 * 100_000.0).abs() < 1e-6);
        // An expensive house is bounded by the cap (~€922), never a runaway %.
        let dear = j.annual_property_tax(&d, 2_000_000.0, 0.0, 0.0, false);
        assert!((dear - cap).abs() < 1e-6);
        // Holiday and permanent are taxed the same in Sweden.
        assert_eq!(
            j.annual_property_tax(&d, 100_000.0, 0.0, 0.0, true),
            j.annual_property_tax(&d, 100_000.0, 0.0, 0.0, false),
        );
    }

    #[test]
    fn norway_dokumentavgift_on_freehold_only() {
        let d = CostDefaults::for_country(Country::No);
        let j = jurisdiction(Country::No);
        // Selveier freehold: 2.5% dokumentavgift.
        assert!((j.transfer_tax(&d, 4_000_000.0, false) - 100_000.0).abs() < 1e-6);
        // Borettslag/aksje co-op share: none.
        assert_eq!(j.transfer_tax(&d, 4_000_000.0, true), 0.0);
    }

    #[test]
    fn norway_charges_flat_registration_even_on_coop() {
        let d = CostDefaults::for_country(Country::No);
        let j = jurisdiction(Country::No);
        let expected = Norway::TINGLYSING_NOK / d.local_per_eur;
        // The flat tinglysingsgebyr is owed on a co-op share transfer too.
        let coop = j.registration_fees(&d, true, false, 0);
        assert_eq!(coop.len(), 1);
        assert!((coop[0].amount - expected).abs() < 1e-6);
        // Cash buyer: no pantedokument line.
        assert!(!coop.iter().any(|f| f.key == "pantedokument"));
    }

    #[test]
    fn norway_property_tax_is_a_share_of_value() {
        let d = CostDefaults::for_country(Country::No);
        let tax = jurisdiction(Country::No).annual_property_tax(&d, 100_000.0, 0.0, 0.0, false);
        // 0.33% of 70% of value.
        assert!((tax - 100_000.0 * 0.70 * 0.0033).abs() < 1e-6);
    }

    #[test]
    fn denmark_deed_duty_on_freehold_not_coop() {
        let d = CostDefaults::for_country(Country::Dk);
        let j = jurisdiction(Country::Dk);
        // Ejerbolig freehold: 0.6% tinglysningsafgift.
        assert!((j.transfer_tax(&d, 2_000_000.0, false) - 12_000.0).abs() < 1e-6);
        // Andelsbolig co-op: none, and no fixed fee line either.
        assert_eq!(j.transfer_tax(&d, 2_000_000.0, true), 0.0);
        assert!(j.registration_fees(&d, true, false, 0).is_empty());
    }

    #[test]
    fn denmark_levies_both_property_taxes() {
        let d = CostDefaults::for_country(Country::Dk);
        let tax = jurisdiction(Country::Dk).annual_property_tax(&d, 200_000.0, 0.0, 0.0, false);
        let base = 0.80 * 200_000.0;
        let evs = 0.0051 * base; // below the progression threshold
        let grundskyld = 0.0074 * 0.80 * 0.35 * 200_000.0;
        assert!((tax - (evs + grundskyld)).abs() < 1e-6);
        // Denmark's combined property tax outweighs Sweden's capped fee.
        let se = jurisdiction(Country::Se).annual_property_tax(
            &CostDefaults::for_country(Country::Se),
            200_000.0,
            0.0,
            0.0,
            false,
        );
        assert!(tax > se);
    }

    #[test]
    fn iceland_stamp_duty_ignores_holding_form_and_uses_assessed_value() {
        let d = CostDefaults::for_country(Country::Is);
        let j = jurisdiction(Country::Is);
        // 0.8% of the assessed fasteignamat (≈85% of price); no freehold/share split.
        let expected = 400_000.0 * 0.85 * 0.008;
        assert!((j.transfer_tax(&d, 400_000.0, false) - expected).abs() < 1e-6);
        assert_eq!(j.transfer_tax(&d, 400_000.0, true), j.transfer_tax(&d, 400_000.0, false));
    }

    #[test]
    fn every_nordic_country_has_a_jurisdiction() {
        for c in Country::ALL {
            assert_eq!(jurisdiction(c).country(), c);
            assert!(c.cost_calibrated());
        }
    }
}
