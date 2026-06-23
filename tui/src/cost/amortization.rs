use serde::{Deserialize, Serialize};

/// The three Finnish mortgage repayment structures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepaymentType {
    /// Equal principal each month; instalment falls over time. Lowest total interest.
    Tasalyhennys,
    /// Level payment, recomputed at each annual rate reset; term fixed.
    Annuiteetti,
    /// Fixed instalment set at origination; term flexes (possible balloon).
    KiinteaTasaera,
}

/// One month of an amortization schedule, in nominal euros.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MonthRow {
    pub interest: f64,
    pub principal: f64,
    pub balance: f64,
}

/// Level annuity payment for principal `p` at monthly rate `r` over `n` months.
fn annuity_payment(p: f64, r: f64, n: usize) -> f64 {
    if n == 0 {
        return 0.0;
    }
    if r.abs() < 1e-12 {
        return p / n as f64;
    }
    p * r / (1.0 - (1.0 + r).powi(-(n as i32)))
}

/// All-in annual rate for a given loan-year; the last entry repeats past the slice.
fn annual_rate_for(rates: &[f64], year: usize) -> f64 {
    if rates.is_empty() {
        return 0.0;
    }
    rates
        .get(year)
        .copied()
        .unwrap_or_else(|| *rates.last().expect("non-empty"))
}

/// Build a month-by-month amortization schedule.
///
/// `annual_rates` is the all-in nominal annual rate per loan-year (12-mo Euribor
/// plus margin); held constant within each 12-month block (the 12-mo Euribor
/// reset cadence) and the last entry repeats if the slice is shorter than the term.
pub fn amortization_schedule(
    principal: f64,
    annual_rates: &[f64],
    term_years: u32,
    repayment: RepaymentType,
) -> Vec<MonthRow> {
    let n_months = (term_years * 12) as usize;
    if n_months == 0 || principal <= 0.0 {
        return Vec::new();
    }
    let mut rows = Vec::with_capacity(n_months);
    let mut balance = principal;
    let const_principal = principal / n_months as f64;
    let fixed_payment = annuity_payment(principal, annual_rate_for(annual_rates, 0) / 12.0, n_months);
    let mut annuity = 0.0;

    for m in 0..n_months {
        let year = m / 12;
        let r = annual_rate_for(annual_rates, year) / 12.0;
        if matches!(repayment, RepaymentType::Annuiteetti) && m % 12 == 0 {
            annuity = annuity_payment(balance, r, n_months - m);
        }
        let interest = balance * r;
        let mut principal_paid = match repayment {
            RepaymentType::Tasalyhennys => const_principal,
            RepaymentType::Annuiteetti => annuity - interest,
            RepaymentType::KiinteaTasaera => fixed_payment - interest,
        };
        principal_paid = principal_paid.clamp(0.0, balance);
        balance -= principal_paid;
        if balance < 1e-6 {
            balance = 0.0;
        }
        rows.push(MonthRow {
            interest,
            principal: principal_paid,
            balance,
        });
    }
    rows
}
