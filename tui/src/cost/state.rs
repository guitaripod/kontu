//! Headless cost-of-ownership state: the inputs the CLI (and the PNG card and
//! the matcher) assemble for a listing before projecting its TCO. Country-aware
//! through the [`CostDefaults`] it is built from.

use super::{
    CostDefaults, HeatingType, HoldingForm, ModelInputs, Projection, PropertyInputs,
    PurchaseInputs, RepaymentType, WaterSupply,
};
use crate::models::Listing;
use crate::risk::RiskAssessment;

/// Assembled cost-model inputs for one listing.
#[derive(Debug, Clone)]
pub struct CostState {
    pub listing_id: Option<i64>,
    pub price: f64,
    pub debt_free_price: f64,
    pub holding_form: HoldingForm,
    pub ltv: f64,
    pub term_years: u32,
    pub euribor: f64,
    pub margin: f64,
    pub repayment: RepaymentType,
    pub heating: HeatingType,
    pub water: WaterSupply,
    pub building_value: Option<f64>,
    pub is_leisure: bool,
    pub fireplace: bool,
    pub private_road: bool,
    pub ground_rent: f64,
    pub vastike: f64,
    /// Actual total energy bill (€/yr); when set, replaces the modelled heating
    /// + electricity estimate. `None` falls back to the defaults.
    pub electricity: Option<f64>,
    /// Actual annual property tax (€/yr); `None` falls back to the estimate.
    pub kiinteistovero: Option<f64>,
    pub horizon: u32,
    pub real_discount: f64,
    pub general_inflation: f64,
    pub energy_inflation: f64,
    pub resale_growth: f64,
    pub capex: Vec<(u32, f64)>,
}

impl CostState {
    pub fn from_defaults(d: &CostDefaults) -> Self {
        Self {
            listing_id: None,
            price: 150_000.0,
            debt_free_price: 150_000.0,
            holding_form: HoldingForm::Kiinteisto,
            ltv: d.ltv_max,
            term_years: d.loan_term_years,
            euribor: d.euribor_12m,
            margin: d.mortgage_margin,
            repayment: RepaymentType::Annuiteetti,
            heating: HeatingType::Kaukolampo,
            water: WaterSupply::Municipal,
            building_value: None,
            is_leisure: false,
            fireplace: false,
            private_road: false,
            ground_rent: 0.0,
            vastike: 0.0,
            electricity: None,
            kiinteistovero: None,
            horizon: 20,
            real_discount: d.discount_rate_real,
            general_inflation: d.general_inflation,
            energy_inflation: d.energy_inflation,
            resale_growth: d.resale_real_growth,
            capex: Vec::new(),
        }
    }

    /// Seed the model from a listing and its risk assessment.
    pub fn apply_listing(&mut self, l: &Listing, risk: &RiskAssessment, d: &CostDefaults) {
        self.listing_id = Some(l.id);
        if let Some(p) = l.price_eur {
            self.price = p as f64;
        }
        self.debt_free_price = l.debt_free_price_eur.map(|v| v as f64).unwrap_or(self.price);
        self.holding_form = match l.holding_form.as_deref() {
            Some(h) if h.contains("osake") => HoldingForm::AsuntoOsake,
            _ => HoldingForm::Kiinteisto,
        };
        self.heating = l.heating_enum();
        self.water = match l.water_supply.as_deref() {
            Some(w) if w.contains("kaivo") || w.contains("kanto") => WaterSupply::Well,
            _ => WaterSupply::Municipal,
        };
        self.ground_rent = l.ground_rent_eur_yr.map(|v| v as f64).unwrap_or(0.0);
        self.vastike =
            (l.maintenance_charge_eur.unwrap_or(0) + l.financing_charge_eur.unwrap_or(0)) as f64;
        self.electricity = l.electricity_eur_yr.map(|v| v as f64);
        self.kiinteistovero = l.kiinteistovero_eur_yr.map(|v| v as f64);
        self.private_road = l
            .road_access
            .as_deref()
            .map(|r| r.contains("yksityis"))
            .unwrap_or(false);
        self.is_leisure = is_leisure_listing(l);
        self.capex = risk.capex_events();
        let _ = d;
    }

    pub fn project(&self, d: &CostDefaults) -> Projection {
        let purchase = PurchaseInputs {
            price_eur: self.price,
            debt_free_price_eur: self.debt_free_price,
            holding_form: self.holding_form,
            ltv: self.ltv,
            term_years: self.term_years,
            interest_rate: self.euribor + self.margin,
            repayment: self.repayment,
            rate_path: None,
            arrangement_fee_eur: 0.0,
            moving_eur: 1000.0,
            inspection_eur: d.kuntotarkastus_eur,
            mortgage_deeds: 1,
            e_conveyance: false,
        };
        let property = PropertyInputs {
            heating: self.heating,
            water: self.water,
            building_value_eur: self.building_value,
            land_value_eur: None,
            is_leisure: self.is_leisure,
            fireplace: self.fireplace,
            private_road: self.private_road,
            ground_rent_eur_yr: self.ground_rent,
            vastike_eur_mo: self.vastike,
            is_apartment: matches!(self.holding_form, HoldingForm::AsuntoOsake),
            kiinteistovero_eur_yr: self.kiinteistovero,
            insurance_eur_yr: None,
            electricity_eur_yr: self.electricity,
            capex: self.capex.clone(),
        };
        let model = ModelInputs {
            horizon_years: self.horizon,
            real_discount_rate: self.real_discount,
            general_inflation: self.general_inflation,
            energy_inflation: self.energy_inflation,
            resale_real_growth: self.resale_growth,
            seller_commission_pct: d.seller_commission_pct,
        };
        super::project(&purchase, &property, &model, d)
    }
}

/// A mökki / loma property is taxed on the general (leisure) kiinteistövero band.
fn is_leisure_listing(l: &Listing) -> bool {
    let leisure_type = l
        .property_type
        .as_deref()
        .map(|t| t.contains("mökki") || t.contains("mokki") || t.contains("loma"))
        .unwrap_or(false);
    let leisure_use = l
        .intended_use
        .as_deref()
        .map(|u| u.contains("loma"))
        .unwrap_or(false);
    leisure_type || leisure_use
}
