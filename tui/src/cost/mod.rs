//! Total-cost-of-ownership engine for a Finnish house purchase. See `SPEC.md` §3.
//!
//! Convention (locked, asserted in tests): cash flows are projected in NOMINAL
//! euros and discounted at a NOMINAL discount rate derived from a real
//! opportunity-cost rate plus general inflation. A net present value is, by
//! construction, expressed in today's euros — so the output is "real, today's
//! euros" while the projection stays internally consistent. Loan principal is
//! NOT a cost (it converts cash into equity); only interest is.

mod amortization;
mod defaults;
mod recurring;

pub use amortization::{amortization_schedule, MonthRow, RepaymentType};
pub use defaults::CostDefaults;
pub use recurring::{HeatingType, WaterSupply};

use serde::{Deserialize, Serialize};

/// Legal form of the purchase — drives the transfer-tax rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HoldingForm {
    /// Real property (kiinteistö): 3% varainsiirtovero, lainhuuto + kaupanvahvistus apply.
    Kiinteisto,
    /// Housing-company shares (asunto-osake): 1.5% varainsiirtovero.
    AsuntoOsake,
}

/// Everything about the loan and the transaction at t=0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseInputs {
    pub price_eur: f64,
    /// Debt-free price (velaton hinta) — the varainsiirtovero base. Defaults to `price_eur`.
    pub debt_free_price_eur: f64,
    pub holding_form: HoldingForm,
    /// Loan-to-value of the debt-free price (e.g. 0.80).
    pub ltv: f64,
    pub term_years: u32,
    /// All-in nominal annual rate = 12-mo Euribor + margin.
    pub interest_rate: f64,
    pub repayment: RepaymentType,
    /// Optional per-year all-in rate path (Euribor resets). Falls back to `interest_rate`.
    pub rate_path: Option<Vec<f64>>,
    pub arrangement_fee_eur: f64,
    pub moving_eur: f64,
    pub inspection_eur: f64,
    /// Number of mortgage deeds (kiinnitys) to register.
    pub mortgage_deeds: u32,
    /// Whether the deed is signed electronically (kaupanvahvistus is then free).
    pub e_conveyance: bool,
}

/// Everything about the property's running costs and resale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyInputs {
    pub heating: HeatingType,
    pub water: WaterSupply,
    /// Building rebuild value, the base for the maintenance reserve. Defaults to
    /// 0.7 × price when not set.
    pub building_value_eur: Option<f64>,
    /// Taxable land value for kiinteistövero. Defaults to 0.2 × price when not set.
    pub land_value_eur: Option<f64>,
    /// Leisure/holiday property (mökki / intended_use loma): taxed on the higher
    /// general building band rather than the permanent-residence band.
    pub is_leisure: bool,
    pub fireplace: bool,
    pub private_road: bool,
    /// Annual ground rent for a leased plot (vuokratontti), 0 for owned plots.
    pub ground_rent_eur_yr: f64,
    /// Monthly housing-company charge (vastike), 0 for a pure kiinteistö.
    pub vastike_eur_mo: f64,
    /// Explicit overrides; when `None` a default is derived.
    pub kiinteistovero_eur_yr: Option<f64>,
    pub insurance_eur_yr: Option<f64>,
    pub electricity_eur_yr: Option<f64>,
    /// Lumpy renovation outlays as (year_from_now, amount_in_todays_eur).
    pub capex: Vec<(u32, f64)>,
}

/// Macro assumptions for the projection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInputs {
    pub horizon_years: u32,
    /// Real opportunity-cost / discount rate (expected real after-tax portfolio return).
    pub real_discount_rate: f64,
    pub general_inflation: f64,
    /// Inflation for energy/water lines (escalates faster than CPI).
    pub energy_inflation: f64,
    /// Real annual change in resale value (rural/lakeside often ~0 or negative).
    pub resale_real_growth: f64,
    pub seller_commission_pct: f64,
}

impl ModelInputs {
    /// Nominal discount rate = (1 + real)(1 + inflation) − 1.
    pub fn nominal_discount(&self) -> f64 {
        (1.0 + self.real_discount_rate) * (1.0 + self.general_inflation) - 1.0
    }

    /// Nominal annual resale growth from the real growth assumption.
    pub fn nominal_resale_growth(&self) -> f64 {
        (1.0 + self.resale_real_growth) * (1.0 + self.general_inflation) - 1.0
    }
}

/// Itemized one-time costs at t=0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OneTimeCosts {
    pub down_payment: f64,
    pub transfer_tax: f64,
    pub lainhuuto: f64,
    pub kaupanvahvistus: f64,
    pub kiinnitys: f64,
    pub inspection: f64,
    pub arrangement_fee: f64,
    pub moving: f64,
}

impl OneTimeCosts {
    pub fn total(&self) -> f64 {
        self.down_payment
            + self.transfer_tax
            + self.lainhuuto
            + self.kaupanvahvistus
            + self.kiinnitys
            + self.inspection
            + self.arrangement_fee
            + self.moving
    }
}

/// One year of the projection, in nominal euros plus the discounted contribution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct YearProjection {
    pub year: u32,
    pub interest: f64,
    pub recurring: f64,
    pub capex: f64,
    pub total_nominal: f64,
    pub discounted: f64,
}

/// The full result of a TCO projection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Projection {
    pub one_time: OneTimeCosts,
    pub years: Vec<YearProjection>,
    /// Resale value − remaining loan − commission at the horizon (nominal).
    pub terminal_equity: f64,
    /// Net present cost in today's euros (positive = cost).
    pub npv_cost: f64,
    /// Level monthly payment whose PV over the horizon equals `npv_cost`.
    pub equivalent_monthly: f64,
    pub total_loan_interest: f64,
    pub loan_principal: f64,
}

fn rate_path(p: &PurchaseInputs) -> Vec<f64> {
    match &p.rate_path {
        Some(path) if !path.is_empty() => path.clone(),
        _ => vec![p.interest_rate; p.term_years.max(1) as usize],
    }
}

/// Compute the itemized one-time costs at t=0.
pub fn one_time_costs(p: &PurchaseInputs, d: &CostDefaults) -> OneTimeCosts {
    let loan = p.price_eur * p.ltv;
    let down_payment = (p.price_eur - loan).max(0.0);
    let transfer_rate = match p.holding_form {
        HoldingForm::Kiinteisto => d.transfer_tax_kiinteisto,
        HoldingForm::AsuntoOsake => d.transfer_tax_osake,
    };
    let is_real_property = matches!(p.holding_form, HoldingForm::Kiinteisto);
    OneTimeCosts {
        down_payment,
        transfer_tax: p.debt_free_price_eur * transfer_rate,
        lainhuuto: if is_real_property { d.lainhuuto_eur } else { 0.0 },
        kaupanvahvistus: if is_real_property {
            if p.e_conveyance {
                d.kaupanvahvistus_econveyance_eur
            } else {
                d.kaupanvahvistus_eur
            }
        } else {
            0.0
        },
        kiinnitys: d.kiinnitys_eur * p.mortgage_deeds as f64,
        inspection: p.inspection_eur,
        arrangement_fee: p.arrangement_fee_eur,
        moving: p.moving_eur,
    }
}

/// Project the total cost of ownership over the horizon.
pub fn project(
    purchase: &PurchaseInputs,
    property: &PropertyInputs,
    model: &ModelInputs,
    defaults: &CostDefaults,
) -> Projection {
    let one_time = one_time_costs(purchase, defaults);
    let loan = purchase.price_eur * purchase.ltv;

    let path = rate_path(purchase);
    let schedule = amortization_schedule(loan, &path, purchase.term_years, purchase.repayment);
    let annual_interest = annual_interest(&schedule);
    let total_loan_interest: f64 = schedule.iter().map(|m| m.interest).sum();

    let building_value = property
        .building_value_eur
        .unwrap_or(0.7 * purchase.price_eur);
    let building_taxable = 0.5 * building_value;
    let land_taxable = property
        .land_value_eur
        .unwrap_or(0.2 * purchase.price_eur);
    let kiinteistovero = property.kiinteistovero_eur_yr.unwrap_or_else(|| {
        defaults.estimated_kiinteistovero(building_taxable, land_taxable, property.is_leisure)
    });
    let lines =
        recurring::recurring_lines(property, model, defaults, building_value, kiinteistovero);
    let nd = model.nominal_discount();

    let mut years = Vec::with_capacity(model.horizon_years as usize);
    let mut npv_flows = 0.0;
    for t in 1..=model.horizon_years {
        let idx = (t - 1) as usize;
        let interest = annual_interest.get(idx).copied().unwrap_or(0.0);
        let recurring: f64 = lines
            .iter()
            .map(|l| l.annual0 * (1.0 + l.inflation).powi((t - 1) as i32))
            .sum();
        let capex: f64 = property
            .capex
            .iter()
            .filter(|(yr, _)| *yr == t)
            .map(|(_, amt)| amt * (1.0 + model.general_inflation).powi((t - 1) as i32))
            .sum();
        let total_nominal = interest + recurring + capex;
        let discounted = total_nominal / (1.0 + nd).powi(t as i32);
        npv_flows += discounted;
        years.push(YearProjection {
            year: t,
            interest,
            recurring,
            capex,
            total_nominal,
            discounted,
        });
    }

    let n = model.horizon_years;
    let remaining_balance = balance_after_months(&schedule, (n * 12) as usize);
    let resale = purchase.price_eur * (1.0 + model.nominal_resale_growth()).powi(n as i32);
    let commission = resale * model.seller_commission_pct;
    let terminal_equity = resale - remaining_balance - commission;
    let discounted_terminal = terminal_equity / (1.0 + nd).powi(n as i32);

    let npv_cost = one_time.total() + npv_flows - discounted_terminal;
    let equivalent_monthly = level_monthly_payment(npv_cost, nd, n);

    Projection {
        one_time,
        years,
        terminal_equity,
        npv_cost,
        equivalent_monthly,
        total_loan_interest,
        loan_principal: loan,
    }
}

/// Sum interest into calendar years (12 months each).
fn annual_interest(schedule: &[MonthRow]) -> Vec<f64> {
    schedule
        .chunks(12)
        .map(|chunk| chunk.iter().map(|m| m.interest).sum())
        .collect()
}

/// Outstanding balance after `months` payments (0 if the schedule is shorter).
fn balance_after_months(schedule: &[MonthRow], months: usize) -> f64 {
    if months == 0 {
        return schedule.first().map(|_| 0.0).unwrap_or(0.0);
    }
    match schedule.get(months.saturating_sub(1)) {
        Some(row) => row.balance,
        None => schedule.last().map(|m| m.balance).unwrap_or(0.0),
    }
}

/// Level monthly payment whose present value over `years` at annual rate `nd`
/// equals `present_value`.
fn level_monthly_payment(present_value: f64, nd: f64, years: u32) -> f64 {
    let n = (years * 12) as f64;
    if n == 0.0 {
        return 0.0;
    }
    let monthly = (1.0 + nd).powf(1.0 / 12.0) - 1.0;
    if monthly.abs() < 1e-12 {
        return present_value / n;
    }
    present_value * monthly / (1.0 - (1.0 + monthly).powf(-n))
}

#[cfg(test)]
mod tests;
