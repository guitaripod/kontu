//! Buyer-risk model (see `SPEC.md` §4 and `docs/expansion/<c>.md` §4). kontu is
//! the abstraction; each Nordic country supplies its own era/structural defect
//! pathology through a [`RiskModel`] (the valesokkeli-class signature flags),
//! while the mechanical flags (pipe age, drainage, roof, oil heating, off-grid
//! sewage, condition, inspection) are shared and country-neutral. Produces a
//! 0–100 RiskScore plus a deferred-capex estimate (€) that feeds the cost engine.

use serde::{Deserialize, Serialize};

use crate::country::Country;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskBand {
    Low,
    Moderate,
    High,
    Severe,
}

impl RiskBand {
    fn from_score(score: u32) -> Self {
        match score {
            0..=24 => RiskBand::Low,
            25..=49 => RiskBand::Moderate,
            50..=74 => RiskBand::High,
            _ => RiskBand::Severe,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            RiskBand::Low => "low",
            RiskBand::Moderate => "moderate",
            RiskBand::High => "high",
            RiskBand::Severe => "severe",
        }
    }
}

/// One contributing risk factor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskFlag {
    pub label: String,
    pub points: u32,
    /// Estimated deferred capital expenditure in today's euros (0 if not a capex item).
    pub capex_eur: f64,
}

impl RiskFlag {
    fn new(label: &str, points: u32, capex_eur: f64) -> Self {
        RiskFlag { label: label.to_string(), points, capex_eur }
    }
}

/// Inputs to the risk model — decoupled from the listing shape so it is testable
/// in isolation. Map a `Listing` into this in the model layer.
#[derive(Debug, Clone, Default)]
pub struct RiskInput {
    /// Which country's pathology applies.
    pub country: Country,
    pub build_year: Option<i32>,
    /// Normalized risk-structure tokens (e.g. "valesokkeli", "enstegstatad", "mgo").
    pub risk_structures: Vec<String>,
    pub heating: Option<String>,
    pub roof_material: Option<String>,
    pub roof_year: Option<i32>,
    pub condition_class: Option<String>,
    pub inspection_done: bool,
    pub sewer_system: Option<String>,
    /// Property is within ~100 m of water or on a groundwater area (stricter sewage rules).
    pub near_water_or_groundwater: bool,
    pub pipes_renovated_year: Option<i32>,
    /// The listing prose states the pipes were renewed, but no year could be parsed.
    /// Suppresses the "original pipes assumed" penalty — evidence, just undated.
    pub pipes_renovation_mentioned: bool,
}

/// Result of a risk assessment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub score: u32,
    pub band: RiskBand,
    pub flags: Vec<RiskFlag>,
    pub deferred_capex_eur: f64,
}

impl RiskAssessment {
    /// Lumpy capex events for the cost engine, scheduling each estimate roughly
    /// within the early ownership window where such projects typically land.
    pub fn capex_events(&self) -> Vec<(u32, f64)> {
        self.flags
            .iter()
            .filter(|f| f.capex_eur > 0.0)
            .enumerate()
            .map(|(i, f)| ((i as u32 % 5) + 2, f.capex_eur))
            .collect()
    }
}

fn has(tokens: &[String], needle: &str) -> bool {
    tokens.iter().any(|t| t.to_lowercase().contains(needle))
}

/// A country's era/structural defect pathology — the signature flags that differ
/// by national building stock (FI valesokkeli, SE enstegstätad fasad, NO wood-rot
/// band, DK MgO-board, IS ASR concrete) plus that country's asbestos/era cutoffs.
pub trait RiskModel {
    fn era_flags(&self, input: &RiskInput, current_year: i32) -> Vec<RiskFlag>;
}

/// Resolve the risk model for a country.
pub fn model(country: Country) -> &'static dyn RiskModel {
    match country {
        Country::Fi => &Finland,
        Country::Se => &Sweden,
        Country::No => &Norway,
        Country::Dk => &Denmark,
        Country::Is => &Iceland,
    }
}

pub struct Finland;
impl RiskModel for Finland {
    fn era_flags(&self, input: &RiskInput, _current_year: i32) -> Vec<RiskFlag> {
        let mut flags = Vec::new();
        let rs = &input.risk_structures;
        if let Some(year) = input.build_year {
            if (1960..=1990).contains(&year) && has(rs, "valesokkeli") {
                flags.push(RiskFlag::new(
                    "Valesokkeli on a 1960–1990 frame — demands a structure-opening kuntotutkimus",
                    30,
                    25_000.0,
                ));
            }
            if year < 1994 {
                flags.push(RiskFlag::new(
                    "Pre-1994 build — asbestos-suspect (haitta-ainekartoitus before renovation)",
                    4,
                    0.0,
                ));
            }
            if (1960..=1985).contains(&year) && !has(rs, "valesokkeli") {
                flags.push(RiskFlag::new("Construction-era (1960–1985) risk structures likely", 8, 0.0));
            }
        }
        flags
    }
}

pub struct Sweden;
impl RiskModel for Sweden {
    fn era_flags(&self, input: &RiskInput, _current_year: i32) -> Vec<RiskFlag> {
        let mut flags = Vec::new();
        let rs = &input.risk_structures;
        if let Some(y) = input.build_year {
            let enstegs = has(rs, "enstegs") || has(rs, "putsad");
            if (1990..=2007).contains(&y) && enstegs {
                flags.push(RiskFlag::new(
                    "Enstegstätad putsad fasad (1990–2007) — Sweden's fuktskandal; structure-opening inspection",
                    25,
                    35_000.0,
                ));
            } else if (1990..=2007).contains(&y) {
                flags.push(RiskFlag::new(
                    "Rendered-façade era (1990–2007) — verify a drained, ventilated cavity",
                    8,
                    0.0,
                ));
            }
            if (1929..=1975).contains(&y) && (has(rs, "blabetong") || has(rs, "radon")) {
                flags.push(RiskFlag::new(
                    "Blåbetong (1929–1975) — radon source; ventilation upgrade",
                    8,
                    3_000.0,
                ));
            }
            if y < 1982 {
                flags.push(RiskFlag::new("Pre-1982 build — asbestos-suspect", 4, 0.0));
            }
        }
        flags
    }
}

pub struct Norway;
impl RiskModel for Norway {
    fn era_flags(&self, input: &RiskInput, _current_year: i32) -> Vec<RiskFlag> {
        let mut flags = Vec::new();
        let rs = &input.risk_structures;
        if let Some(y) = input.build_year {
            if (1970..=1995).contains(&y) {
                flags.push(RiskFlag::new(
                    "Wood-frame moisture/rot band (1970–1995) — Norway's high-risk era; check sill plate & drainage",
                    12,
                    13_000.0,
                ));
            }
            if (2000..=2010).contains(&y) && (has(rs, "puss") || has(rs, "etics") || has(rs, "render")) {
                flags.push(RiskFlag::new(
                    "Render-on-insulation façade (~2000–2010) — moisture risk without a ventilated cavity",
                    15,
                    25_000.0,
                ));
            }
            if y <= 1985 {
                flags.push(RiskFlag::new(
                    "Pre-1985 build — asbestos-suspect (eternit/products banned 1985)",
                    4,
                    0.0,
                ));
            }
            if (1960..=1980).contains(&y) {
                flags.push(RiskFlag::new(
                    "1960–1980 build — PCB-suspect materials (survey before renovation)",
                    3,
                    0.0,
                ));
            }
        }
        flags
    }
}

pub struct Denmark;
impl RiskModel for Denmark {
    fn era_flags(&self, input: &RiskInput, _current_year: i32) -> Vec<RiskFlag> {
        let mut flags = Vec::new();
        let rs = &input.risk_structures;
        if let Some(y) = input.build_year {
            let mgo = has(rs, "mgo") || has(rs, "magnesium");
            if (2010..=2015).contains(&y) && mgo {
                flags.push(RiskFlag::new(
                    "MgO wind-barrier board (2010–2015) — Denmark's façade scandal; moisture-failed boards",
                    25,
                    33_000.0,
                ));
            } else if (2010..=2015).contains(&y) {
                flags.push(RiskFlag::new(
                    "MgO-board era (2010–2015) — verify the wind-barrier board is not MgO",
                    8,
                    0.0,
                ));
            }
            if (1960..=1979).contains(&y) {
                flags.push(RiskFlag::new(
                    "1960–1979 build — gasbeton / light timber-frame defects of the era",
                    8,
                    0.0,
                ));
            }
            if y < 1990 {
                flags.push(RiskFlag::new("Pre-1990 build — asbestos-suspect (eternit ban 1986)", 4, 0.0));
            }
        }
        flags
    }
}

pub struct Iceland;
impl RiskModel for Iceland {
    fn era_flags(&self, input: &RiskInput, _current_year: i32) -> Vec<RiskFlag> {
        let mut flags = Vec::new();
        let rs = &input.risk_structures;
        if let Some(y) = input.build_year {
            // Iceland's stock is cast-in-place reinforced concrete; silica fume
            // was intermilled into all cement from 1979, ending ASR-prone casts.
            if (1961..=1979).contains(&y) {
                flags.push(RiskFlag::new(
                    "ASR (alkalívirkni) concrete (1961–1979) — Iceland's signature defect; inspect for map-cracking",
                    25,
                    40_000.0,
                ));
            }
            if y < 1976 && (has(rs, "sisz") || has(rs, "seismic")) {
                flags.push(RiskFlag::new(
                    "Pre-1976 build in a seismic zone — predates the 1976 seismic code (ÍST 13)",
                    10,
                    0.0,
                ));
            }
            if y < 1984 {
                flags.push(RiskFlag::new("Pre-1984 build — asbestos-suspect (banned 1983)", 4, 0.0));
            }
            // Radon contributes 0 in Iceland (basaltic bedrock) — no flag.
        }
        flags
    }
}

fn roof_lifespan(material: &str) -> i32 {
    let m = material.to_lowercase();
    if m.contains("huopa") || m.contains("papp") || m.contains("tagpap") || m.contains("felt") {
        30
    } else if m.contains("pelti") || m.contains("metal") || m.contains("plåt") || m.contains("bárujárn") {
        45
    } else if m.contains("tiili") || m.contains("tegl") || m.contains("clay") || m.contains("betong") {
        50
    } else {
        35
    }
}

/// True when a token names an era/structural defect already scored by a country
/// model or a dedicated shared flag, so the generic catch-all does not double-count it.
fn is_signature_token(t: &str) -> bool {
    const KNOWN: [&str; 14] = [
        "valesokkeli", "salaoj", "enstegs", "putsad", "blabetong", "radon", "mgo", "magnesium",
        "puss", "etics", "render", "sisz", "seismic", "steinsteyp",
    ];
    KNOWN.iter().any(|k| t.contains(k))
}

/// Assess buyer risk for a property as of `current_year`. Dispatches the era
/// pathology to the country's [`RiskModel`]; the rest is shared.
pub fn assess(input: &RiskInput, current_year: i32) -> RiskAssessment {
    let mut flags = model(input.country).era_flags(input, current_year);
    let age = input.build_year.map(|y| current_year - y);

    pipe_flags(input, current_year, &mut flags);

    if let Some(a) = age {
        if a >= 35 || has(&input.risk_structures, "salaoj") {
            flags.push(RiskFlag::new("Foundation drainage likely due", 8, 12_000.0));
        }
    }

    match (input.roof_year, input.roof_material.as_deref(), age) {
        (Some(ry), Some(material), _) if current_year - ry > roof_lifespan(material) => {
            flags.push(RiskFlag::new("Roof past its expected lifespan", 10, 18_000.0));
        }
        (None, _, Some(a)) if a > 35 => {
            flags.push(RiskFlag::new("Roof age unknown on an aging house", 5, 0.0));
        }
        _ => {}
    }

    if is_oil_heating(input.heating.as_deref()) {
        flags.push(RiskFlag::new(
            "Oil heating — phase-out + high running cost (conversion advised)",
            8,
            0.0,
        ));
    }

    sewage_flags(input, &mut flags);
    condition_flags(input, &mut flags);

    if !input.inspection_done {
        flags.push(RiskFlag::new("No condition inspection on record", 6, 0.0));
    }

    for token in &input.risk_structures {
        let t = token.to_lowercase();
        if is_signature_token(&t) {
            continue;
        }
        flags.push(RiskFlag::new(&format!("Risk structure: {token}"), 10, 10_000.0));
    }

    let raw: u32 = flags.iter().map(|f| f.points).sum();
    let score = raw.min(100);
    let deferred_capex_eur = flags.iter().map(|f| f.capex_eur).sum();

    RiskAssessment {
        score,
        band: RiskBand::from_score(score),
        flags,
        deferred_capex_eur,
    }
}

/// Score the EFFECTIVE pipe age — years since the last renewal, or since the
/// build if never renewed. A known renewal year is a fact, scored at full weight
/// (and an old known renewal IS flagged). An unrecorded one is an UNKNOWN:
/// provisioned for cost but scored lighter and labelled "verify", so the model
/// never sells a guess as a confident defect. Renewal stated in the prose without
/// a year suppresses the assumption. (Shared across the Nordics — every country
/// has the same pipe-renewal lifecycle, only the local name differs.)
fn pipe_flags(input: &RiskInput, current_year: i32, flags: &mut Vec<RiskFlag>) {
    let Some(ref_year) = input.pipes_renovated_year.or(input.build_year) else {
        return;
    };
    let pipe_age = current_year - ref_year;
    if input.pipes_renovated_year.is_some() {
        if pipe_age > 40 {
            flags.push(RiskFlag::new(
                &format!("Pipe renewal overdue ({pipe_age} yr since last renewal)"),
                18,
                30_000.0,
            ));
        } else if pipe_age >= 30 {
            flags.push(RiskFlag::new(
                &format!("Pipe renewal approaching ({pipe_age} yr since renewal)"),
                8,
                15_000.0,
            ));
        }
    } else if input.pipes_renovation_mentioned {
        if pipe_age > 30 {
            flags.push(RiskFlag::new(
                "Pipe renewal noted in listing but year unconfirmed — verify in inspection",
                3,
                0.0,
            ));
        }
    } else if pipe_age > 40 {
        flags.push(RiskFlag::new(
            "Pipe renewal likely due — original pipes assumed (no renewal on record; verify in inspection)",
            10,
            30_000.0,
        ));
    } else if pipe_age >= 30 {
        flags.push(RiskFlag::new(
            "Pipe renewal approaching — pipe age unrecorded (verify in inspection)",
            5,
            15_000.0,
        ));
    }
}

/// Basic off-grid sewage (septic tank / holding tank, by any Nordic name) near
/// water or groundwater is a likely upgrade; the jätevesi/enskilt avlopp/
/// spildevand/spredt avløp analogue.
fn sewage_flags(input: &RiskInput, flags: &mut Vec<RiskFlag>) {
    let Some(sewer) = input.sewer_system.as_deref() else {
        return;
    };
    let s = sewer.to_lowercase();
    let basic = ["saostus", "umpis", "septik", "trekammar", "bundfald", "slamav", "rotþro", "rotthro"]
        .iter()
        .any(|k| s.contains(k));
    if basic && input.near_water_or_groundwater {
        flags.push(RiskFlag::new(
            "Off-grid sewage upgrade likely (basic system near water / groundwater)",
            12,
            12_000.0,
        ));
    } else if basic {
        flags.push(RiskFlag::new("Basic off-grid sewage — verify compliance", 5, 6_000.0));
    }
}

fn condition_flags(input: &RiskInput, flags: &mut Vec<RiskFlag>) {
    let Some(cond) = input.condition_class.as_deref() else {
        return;
    };
    let c = cond.to_lowercase();
    let poor = c.contains("huono") || c.contains("poor") || c.contains("critical") || c.contains("dålig") || c.contains("darlig");
    let fair = c.contains("vältt") || c.contains("valtt") || c.contains("fair") || c.contains("serious");
    if poor {
        flags.push(RiskFlag::new("Condition rated poor", 15, 0.0));
    } else if fair {
        flags.push(RiskFlag::new("Condition rated fair", 8, 0.0));
    }
}

fn is_oil_heating(heating: Option<&str>) -> bool {
    let Some(h) = heating else { return false };
    let h = h.to_lowercase();
    ["oljy", "öljy", "olja", "olje", "oil", "olíu", "oliu", "oliefyr", "parafin"]
        .iter()
        .any(|k| h.contains(k))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_house() -> RiskInput {
        RiskInput {
            build_year: Some(2021),
            inspection_done: true,
            heating: Some("maalampo".into()),
            ..Default::default()
        }
    }

    #[test]
    fn new_house_is_low_risk() {
        let a = assess(&new_house(), 2026);
        assert_eq!(a.band, RiskBand::Low);
        assert!(a.score < 25, "score {}", a.score);
        assert_eq!(a.deferred_capex_eur, 0.0);
    }

    #[test]
    fn valesokkeli_1975_is_high_risk_with_capex() {
        let input = RiskInput {
            build_year: Some(1975),
            risk_structures: vec!["valesokkeli".into()],
            inspection_done: false,
            heating: Some("oljy".into()),
            ..Default::default()
        };
        let a = assess(&input, 2026);
        assert!(a.score >= 50, "score {}", a.score);
        assert!(matches!(a.band, RiskBand::High | RiskBand::Severe));
        assert!(a.deferred_capex_eur >= 25_000.0);
        assert!(a.flags.iter().any(|f| f.label.contains("Valesokkeli")));
        assert!(a.flags.iter().any(|f| f.label.contains("Oil heating")));
    }

    #[test]
    fn old_pipes_flag_capex() {
        let input = RiskInput { build_year: Some(1980), inspection_done: true, ..Default::default() };
        let a = assess(&input, 2026);
        assert!(a.flags.iter().any(|f| f.label.contains("Pipe renewal")));
        assert!(a.deferred_capex_eur > 0.0);
    }

    #[test]
    fn renovated_pipes_suppress_overdue_flag() {
        let input = RiskInput {
            build_year: Some(1980),
            pipes_renovated_year: Some(2015),
            inspection_done: true,
            ..Default::default()
        };
        let a = assess(&input, 2026);
        assert!(!a.flags.iter().any(|f| f.label.contains("overdue")));
    }

    /// A *known* renewal that is itself decades old must still flag overdue.
    #[test]
    fn old_known_pipe_renewal_still_flags_overdue() {
        let input = RiskInput {
            build_year: Some(1965),
            pipes_renovated_year: Some(1982),
            inspection_done: true,
            ..Default::default()
        };
        let a = assess(&input, 2026);
        let pipe = a.flags.iter().find(|f| f.label.contains("Pipe renewal")).expect("pipe flag");
        assert!(pipe.label.contains("overdue"), "got {:?}", pipe.label);
        assert_eq!(pipe.points, 18);
    }

    /// An UNKNOWN pipe age must not be sold as a confident €30k known defect.
    #[test]
    fn unrecorded_pipe_age_is_lighter_and_labelled_unknown() {
        let known_overdue = assess(
            &RiskInput { build_year: Some(1965), pipes_renovated_year: Some(1982), inspection_done: true, ..Default::default() },
            2026,
        );
        let unknown = assess(
            &RiskInput { build_year: Some(1970), inspection_done: true, ..Default::default() },
            2026,
        );
        let kp = known_overdue.flags.iter().find(|f| f.label.contains("Pipe renewal")).unwrap();
        let up = unknown.flags.iter().find(|f| f.label.contains("Pipe renewal")).unwrap();
        assert!(up.points < kp.points, "unknown ({}) must score below known-overdue ({})", up.points, kp.points);
        assert!(up.label.contains("verify"), "unknown flag must read as a verify item: {:?}", up.label);
    }

    /// Renewal stated in the prose (but undated) suppresses the "original pipes" assumption.
    #[test]
    fn mentioned_pipe_renewal_suppresses_the_assumption() {
        let input = RiskInput {
            build_year: Some(1970),
            pipes_renovation_mentioned: true,
            inspection_done: true,
            ..Default::default()
        };
        let a = assess(&input, 2026);
        assert!(!a.flags.iter().any(|f| f.label.contains("original pipes assumed")));
        assert!(a.flags.iter().all(|f| !f.label.contains("Pipe renewal") || f.capex_eur == 0.0));
    }

    #[test]
    fn sewage_near_water_flags_upgrade() {
        let input = RiskInput {
            build_year: Some(2000),
            sewer_system: Some("saostuskaivo".into()),
            near_water_or_groundwater: true,
            inspection_done: true,
            ..Default::default()
        };
        let a = assess(&input, 2026);
        assert!(a.flags.iter().any(|f| f.label.contains("sewage")));
    }

    #[test]
    fn capex_events_schedule_within_horizon() {
        let input = RiskInput {
            build_year: Some(1975),
            risk_structures: vec!["valesokkeli".into()],
            ..Default::default()
        };
        let events = assess(&input, 2026).capex_events();
        assert!(!events.is_empty());
        assert!(events.iter().all(|(yr, _)| *yr >= 2 && *yr <= 6));
    }

    #[test]
    fn score_clamps_at_100() {
        let input = RiskInput {
            build_year: Some(1965),
            risk_structures: vec!["valesokkeli".into(), "kaksoislaatta".into(), "kosteus".into()],
            sewer_system: Some("umpisailio".into()),
            near_water_or_groundwater: true,
            condition_class: Some("huono".into()),
            heating: Some("oljy".into()),
            inspection_done: false,
            ..Default::default()
        };
        let a = assess(&input, 2026);
        assert!(a.score <= 100);
        assert_eq!(a.band, RiskBand::Severe);
    }

    #[test]
    fn sweden_enstegstatad_is_the_signature_flag() {
        let input = RiskInput {
            country: Country::Se,
            build_year: Some(2003),
            risk_structures: vec!["enstegstatad".into()],
            inspection_done: true,
            ..Default::default()
        };
        let a = assess(&input, 2026);
        assert!(a.flags.iter().any(|f| f.label.contains("Enstegstätad")));
        assert!(a.deferred_capex_eur >= 35_000.0);
        // Finland's valesokkeli must NOT appear for a Swedish house.
        assert!(!a.flags.iter().any(|f| f.label.contains("Valesokkeli")));
    }

    #[test]
    fn denmark_mgo_board_is_the_signature_flag() {
        let input = RiskInput {
            country: Country::Dk,
            build_year: Some(2013),
            risk_structures: vec!["mgo".into()],
            inspection_done: true,
            ..Default::default()
        };
        let a = assess(&input, 2026);
        assert!(a.flags.iter().any(|f| f.label.contains("MgO")));
        assert!(a.deferred_capex_eur >= 33_000.0);
    }

    #[test]
    fn iceland_asr_concrete_flags_without_a_token() {
        // Iceland's stock is concrete by default — the era alone flags ASR.
        let input = RiskInput {
            country: Country::Is,
            build_year: Some(1970),
            inspection_done: true,
            ..Default::default()
        };
        let a = assess(&input, 2026);
        assert!(a.flags.iter().any(|f| f.label.contains("ASR")));
        // No Finnish valesokkeli, no radon flag for Iceland.
        assert!(!a.flags.iter().any(|f| f.label.contains("Valesokkeli")));
    }

    #[test]
    fn norway_wood_rot_band_flags_the_era() {
        let input = RiskInput {
            country: Country::No,
            build_year: Some(1985),
            inspection_done: true,
            ..Default::default()
        };
        let a = assess(&input, 2026);
        assert!(a.flags.iter().any(|f| f.label.contains("Wood-frame moisture/rot")));
    }
}
