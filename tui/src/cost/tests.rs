use super::*;
use super::recurring::recurring_lines;

fn approx(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

/// Amount of a registration fee line by key (0 if absent).
fn fee(ot: &OneTimeCosts, key: &str) -> f64 {
    ot.registration_fees.iter().find(|f| f.key == key).map(|f| f.amount).unwrap_or(0.0)
}

fn sample_purchase() -> PurchaseInputs {
    PurchaseInputs {
        price_eur: 200_000.0,
        debt_free_price_eur: 200_000.0,
        holding_form: HoldingForm::Kiinteisto,
        ltv: 0.80,
        term_years: 25,
        interest_rate: 0.0333,
        repayment: RepaymentType::Annuiteetti,
        rate_path: None,
        arrangement_fee_eur: 400.0,
        moving_eur: 800.0,
        inspection_eur: 1450.0,
        mortgage_deeds: 1,
        e_conveyance: false,
    }
}

fn sample_property() -> PropertyInputs {
    PropertyInputs {
        heating: HeatingType::Kaukolampo,
        water: WaterSupply::Municipal,
        building_value_eur: Some(140_000.0),
        land_value_eur: None,
        is_leisure: false,
        fireplace: false,
        private_road: false,
        ground_rent_eur_yr: 0.0,
        vastike_eur_mo: 0.0,
        is_apartment: false,
        kiinteistovero_eur_yr: None,
        insurance_eur_yr: None,
        electricity_eur_yr: None,
        capex: vec![],
    }
}

fn sample_model() -> ModelInputs {
    ModelInputs {
        horizon_years: 20,
        real_discount_rate: 0.03,
        general_inflation: 0.02,
        energy_inflation: 0.04,
        resale_real_growth: 0.0,
        seller_commission_pct: 0.035,
    }
}

#[test]
fn nominal_discount_compounds_real_and_inflation() {
    let m = sample_model();
    assert!(approx(m.nominal_discount(), 0.0506, 1e-9));
}

#[test]
fn annuity_schedule_known_payment_and_full_amortization() {
    let sched = amortization_schedule(200_000.0, &[0.03], 25, RepaymentType::Annuiteetti);
    assert_eq!(sched.len(), 300);
    let total_principal: f64 = sched.iter().map(|m| m.principal).sum();
    assert!(approx(total_principal, 200_000.0, 1.0), "principal {total_principal}");
    assert!(sched.last().unwrap().balance < 1.0);
    let payment0 = sched[0].interest + sched[0].principal;
    assert!(approx(payment0, 948.4, 1.0), "payment {payment0}");
}

#[test]
fn tasalyhennys_cheaper_than_annuiteetti() {
    let tas: f64 = amortization_schedule(200_000.0, &[0.03], 25, RepaymentType::Tasalyhennys)
        .iter()
        .map(|m| m.interest)
        .sum();
    let ann: f64 = amortization_schedule(200_000.0, &[0.03], 25, RepaymentType::Annuiteetti)
        .iter()
        .map(|m| m.interest)
        .sum();
    assert!(tas < ann, "tasalyhennys {tas} should be < annuiteetti {ann}");
    assert!(approx(ann, 84_508.0, 600.0), "annuiteetti total interest {ann}");
}

#[test]
fn tasalyhennys_constant_principal_and_amortizes() {
    let sched = amortization_schedule(120_000.0, &[0.04], 20, RepaymentType::Tasalyhennys);
    assert!(approx(sched[0].principal, sched.last().unwrap().principal, 1e-6));
    assert!(sched.last().unwrap().balance < 1.0);
}

#[test]
fn rate_path_resets_raise_interest() {
    let flat: f64 = amortization_schedule(200_000.0, &[0.03], 25, RepaymentType::Annuiteetti)
        .iter()
        .map(|m| m.interest)
        .sum();
    let mut rising = vec![0.03; 25];
    for (i, r) in rising.iter_mut().enumerate() {
        *r = 0.03 + 0.001 * i as f64;
    }
    let rose: f64 = amortization_schedule(200_000.0, &rising, 25, RepaymentType::Annuiteetti)
        .iter()
        .map(|m| m.interest)
        .sum();
    assert!(rose > flat, "rising-rate interest {rose} should exceed flat {flat}");
}

#[test]
fn one_time_costs_kiinteisto() {
    let d = CostDefaults::default();
    let mut p = sample_purchase();
    p.price_eur = 250_000.0;
    p.debt_free_price_eur = 250_000.0;
    let ot = one_time_costs(&p, &d);
    assert!(approx(ot.down_payment, 50_000.0, 1e-6));
    assert!(approx(ot.transfer_tax, 7_500.0, 1e-6));
    assert!(approx(fee(&ot, "lainhuuto"), 172.0, 1e-6));
    assert!(approx(fee(&ot, "kaupanvahvistus"), 143.0, 1e-6));
    assert!(approx(fee(&ot, "kiinnitys"), 47.0, 1e-6));
    let expected = 50_000.0 + 7_500.0 + 172.0 + 143.0 + 47.0 + 1450.0 + 400.0 + 800.0;
    assert!(approx(ot.total(), expected, 1e-6));
}

#[test]
fn osake_lower_transfer_tax_and_no_realproperty_fees() {
    let d = CostDefaults::default();
    let mut p = sample_purchase();
    p.holding_form = HoldingForm::AsuntoOsake;
    let ot = one_time_costs(&p, &d);
    assert!(approx(ot.transfer_tax, 3_000.0, 1e-6));
    assert_eq!(fee(&ot, "lainhuuto"), 0.0);
    assert_eq!(fee(&ot, "kaupanvahvistus"), 0.0);
    assert!(!ot.registration_fees.iter().any(|f| f.key == "lainhuuto" || f.key == "kaupanvahvistus"));
}

#[test]
fn e_conveyance_zeroes_kaupanvahvistus() {
    let d = CostDefaults::default();
    let mut p = sample_purchase();
    p.e_conveyance = true;
    assert_eq!(fee(&one_time_costs(&p, &d), "kaupanvahvistus"), 0.0);
}

#[test]
fn zero_discount_zero_inflation_npv_is_plain_sum() {
    let d = CostDefaults::default();
    let model = ModelInputs {
        horizon_years: 10,
        real_discount_rate: 0.0,
        general_inflation: 0.0,
        energy_inflation: 0.0,
        resale_real_growth: 0.0,
        seller_commission_pct: 0.0,
    };
    let proj = project(&sample_purchase(), &sample_property(), &model, &d);
    let sum_flows: f64 = proj.years.iter().map(|y| y.total_nominal).sum();
    let expected = proj.one_time.total() + sum_flows - proj.terminal_equity;
    assert!(approx(proj.npv_cost, expected, 1e-6), "npv {} vs {expected}", proj.npv_cost);
    for y in &proj.years {
        assert!(approx(y.discounted, y.total_nominal, 1e-9));
    }
}

#[test]
fn longer_term_increases_npv_via_more_interest() {
    let d = CostDefaults::default();
    let prop = sample_property();
    let model = sample_model();
    let mut p20 = sample_purchase();
    p20.term_years = 20;
    let mut p30 = sample_purchase();
    p30.term_years = 30;
    let n20 = project(&p20, &prop, &model, &d).npv_cost;
    let n30 = project(&p30, &prop, &model, &d).npv_cost;
    assert!(n30 > n20, "30y NPV {n30} should exceed 20y NPV {n20}");
}

#[test]
fn heating_choice_changes_recurring() {
    let d = CostDefaults::default();
    let model = sample_model();
    let p = sample_purchase();
    let mut maa = sample_property();
    maa.heating = HeatingType::Maalampo;
    let mut oil = sample_property();
    oil.heating = HeatingType::Oljy;
    let nmaa = project(&p, &maa, &model, &d).npv_cost;
    let noil = project(&p, &oil, &model, &d).npv_cost;
    assert!(noil > nmaa, "oil {noil} should cost more than ground-source {nmaa}");
}

#[test]
fn capex_inside_horizon_raises_npv_outside_does_not() {
    let d = CostDefaults::default();
    let model = sample_model();
    let p = sample_purchase();
    let base = project(&p, &sample_property(), &model, &d).npv_cost;
    let mut inside = sample_property();
    inside.capex = vec![(3, 30_000.0)];
    assert!(project(&p, &inside, &model, &d).npv_cost > base);
    let mut outside = sample_property();
    outside.capex = vec![(50, 30_000.0)];
    assert!(approx(project(&p, &outside, &model, &d).npv_cost, base, 1e-6));
}

#[test]
fn equivalent_monthly_positive_and_reasonable() {
    let d = CostDefaults::default();
    let proj = project(&sample_purchase(), &sample_property(), &sample_model(), &d);
    assert!(proj.npv_cost > 0.0);
    assert!(proj.equivalent_monthly > 0.0);
    assert!(proj.equivalent_monthly < proj.npv_cost);
}

#[test]
fn capex_year_one_equals_todays_euros() {
    let d = CostDefaults::default();
    let mut prop = sample_property();
    prop.capex = vec![(1, 30_000.0)];
    let proj = project(&sample_purchase(), &prop, &sample_model(), &d);
    assert!(
        approx(proj.years[0].capex, 30_000.0, 1e-6),
        "year-1 capex {} should equal today's euros",
        proj.years[0].capex
    );
}

#[test]
fn kiintea_tasaera_flexes_term_under_rising_rates() {
    let flat: f64 = amortization_schedule(200_000.0, &[0.03], 25, RepaymentType::KiinteaTasaera)
        .iter()
        .map(|m| m.interest)
        .sum();
    let mut rising = vec![0.03; 25];
    for (i, r) in rising.iter_mut().enumerate() {
        *r = 0.03 + 0.003 * i as f64;
    }
    let sched = amortization_schedule(200_000.0, &rising, 25, RepaymentType::KiinteaTasaera);
    let rose: f64 = sched.iter().map(|m| m.interest).sum();
    assert!(rose > flat, "rising kiinteä interest {rose} should exceed flat {flat}");
    assert!(sched.len() > 300, "term should flex past 25y, got {} months", sched.len());
}

#[test]
fn leisure_kiinteistovero_exceeds_permanent() {
    let d = CostDefaults::default();
    let model = sample_model();
    let p = sample_purchase();
    let mut perm = sample_property();
    perm.is_leisure = false;
    let mut leisure = sample_property();
    leisure.is_leisure = true;
    let n_perm = project(&p, &perm, &model, &d).npv_cost;
    let n_leisure = project(&p, &leisure, &model, &d).npv_cost;
    assert!(n_leisure > n_perm, "leisure NPV {n_leisure} should exceed permanent {n_perm}");
}

#[test]
fn actual_electricity_replaces_heating_only_for_electric_heat() {
    let d = CostDefaults::default();
    let m = sample_model();
    let heating_of = |p: &PropertyInputs| {
        let lines = recurring_lines(p, &m, &d, 140_000.0, 900.0);
        let h = lines.iter().find(|l| l.name == "heating").map(|l| l.annual0).unwrap_or(-1.0);
        let e = lines.iter().find(|l| l.name == "electricity").map(|l| l.annual0);
        (h, e)
    };
    // Electric heating: the actual electricity bill IS the heating, so zero it out.
    let mut elec = sample_property();
    elec.heating = HeatingType::Sahko;
    elec.electricity_eur_yr = Some(700.0);
    let (h, e) = heating_of(&elec);
    assert_eq!(h, 0.0, "electric heating is inside the electricity bill");
    assert_eq!(e, Some(700.0));
    // District heating: billed separately, so keep the heating line AND add power.
    let mut dh = sample_property();
    dh.heating = HeatingType::Kaukolampo;
    dh.electricity_eur_yr = Some(700.0);
    let (h, e) = heating_of(&dh);
    assert!(h > 0.0, "district heat is NOT in the electricity bill — keep it");
    assert_eq!(e, Some(700.0), "electricity is added on top of district heat");
    // A zero/garbage reading must fall back to the heating estimate.
    let mut zero = sample_property();
    zero.heating = HeatingType::Sahko;
    zero.electricity_eur_yr = Some(0.0);
    assert!(heating_of(&zero).0 > 0.0, "a 0 reading is not a real figure");
}

#[test]
fn apartment_vastike_replaces_owner_upkeep_lines_no_double_count() {
    let d = CostDefaults::default();
    let m = sample_model();
    let mut flat = sample_property();
    flat.is_apartment = true;
    flat.vastike_eur_mo = 300.0;
    let lines = recurring_lines(&flat, &m, &d, 140_000.0, 900.0);
    let names: Vec<&str> = lines.iter().map(|l| l.name).collect();
    assert!(names.contains(&"vastike"), "apartment must charge vastike");
    for subsumed in ["maintenance_reserve", "kiinteistovero", "water", "waste", "heating"] {
        assert!(!names.contains(&subsumed), "hoitovastike subsumes {subsumed}; must not be added on top");
    }
}

#[test]
fn osake_mortgage_sized_on_purchase_price_not_debt_free() {
    let d = CostDefaults::default();
    let mut p = sample_purchase();
    p.holding_form = HoldingForm::AsuntoOsake;
    p.price_eur = 50_000.0;
    p.debt_free_price_eur = 200_000.0;
    p.ltv = 0.80;
    let ot = one_time_costs(&p, &d);
    assert!(approx(ot.down_payment, 10_000.0, 1e-6), "down payment {}", ot.down_payment);
    assert!(approx(ot.transfer_tax, 3_000.0, 1e-6), "transfer tax {}", ot.transfer_tax);
}
