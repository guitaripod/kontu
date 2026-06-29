//! Finnish house buyer-risk model (see `SPEC.md` §4). Produces a 0–100 RiskScore
//! and a deferred-capex estimate (€) that feeds the cost engine's lumpy capex.
//! Build year is the master multiplier for structural and material risk.

use serde::{Deserialize, Serialize};

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

/// Inputs to the risk model — decoupled from the listing shape so it is testable
/// in isolation. Map a `Listing` into this in the UI layer.
#[derive(Debug, Clone, Default)]
pub struct RiskInput {
    pub build_year: Option<i32>,
    /// Normalized risk-structure tokens (e.g. "valesokkeli").
    pub risk_structures: Vec<String>,
    pub heating: Option<String>,
    pub roof_material: Option<String>,
    pub roof_year: Option<i32>,
    pub condition_class: Option<String>,
    pub inspection_done: bool,
    pub sewer_system: Option<String>,
    /// Property is within ~100 m of water or on a groundwater area (stricter jätevesi rules).
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

fn roof_lifespan(material: &str) -> i32 {
    let m = material.to_lowercase();
    if m.contains("huopa") {
        30
    } else if m.contains("pelti") {
        45
    } else if m.contains("tiili") {
        50
    } else {
        35
    }
}

/// Assess buyer risk for a property as of `current_year`.
pub fn assess(input: &RiskInput, current_year: i32) -> RiskAssessment {
    let mut flags: Vec<RiskFlag> = Vec::new();
    let age = input.build_year.map(|y| current_year - y);

    if let Some(year) = input.build_year {
        if (1960..=1990).contains(&year) && has(&input.risk_structures, "valesokkeli") {
            flags.push(RiskFlag {
                label: "Valesokkeli on a 1960–1990 frame — demands a structure-opening kuntotutkimus".into(),
                points: 30,
                capex_eur: 25_000.0,
            });
        }
        if year < 1994 {
            flags.push(RiskFlag {
                label: "Pre-1994 build — asbestos-suspect (haitta-ainekartoitus before renovation)".into(),
                points: 4,
                capex_eur: 0.0,
            });
        }
        if (1960..=1985).contains(&year) && !has(&input.risk_structures, "valesokkeli") {
            flags.push(RiskFlag {
                label: "Construction-era (1960–1985) risk structures likely".into(),
                points: 8,
                capex_eur: 0.0,
            });
        }
    }

    // Plumbing: score the EFFECTIVE pipe age — years since the last renewal, or
    // since the build if never renewed. A known renewal year is a fact, scored at
    // full weight (and an old known renewal IS flagged, which a mere is_some() check
    // missed). An unrecorded one is an UNKNOWN: provisioned for cost but scored
    // lighter and labelled "verify", so the model never sells a guess as a confident
    // defect. Renewal stated in the prose without a year suppresses the assumption.
    if let Some(ref_year) = input.pipes_renovated_year.or(input.build_year) {
        let pipe_age = current_year - ref_year;
        if input.pipes_renovated_year.is_some() {
            if pipe_age > 40 {
                flags.push(RiskFlag {
                    label: format!("Putkiremontti overdue (pipes {pipe_age} yr since last renewal)"),
                    points: 18,
                    capex_eur: 30_000.0,
                });
            } else if pipe_age >= 30 {
                flags.push(RiskFlag {
                    label: format!("Putkiremontti approaching (pipes {pipe_age} yr since renewal)"),
                    points: 8,
                    capex_eur: 15_000.0,
                });
            }
        } else if input.pipes_renovation_mentioned {
            if pipe_age > 30 {
                flags.push(RiskFlag {
                    label: "Pipe renewal noted in listing but year unconfirmed — verify in kuntotarkastus".into(),
                    points: 3,
                    capex_eur: 0.0,
                });
            }
        } else if pipe_age > 40 {
            flags.push(RiskFlag {
                label: "Putkiremontti likely due — original pipes assumed (no renewal on record; verify in kuntotarkastus)".into(),
                points: 10,
                capex_eur: 30_000.0,
            });
        } else if pipe_age >= 30 {
            flags.push(RiskFlag {
                label: "Putkiremontti approaching — pipe age unrecorded (verify in kuntotarkastus)".into(),
                points: 5,
                capex_eur: 15_000.0,
            });
        }
    }

    if let Some(a) = age {
        if a >= 35 || has(&input.risk_structures, "salaoj") {
            flags.push(RiskFlag {
                label: "Foundation drains (salaojat) likely due".into(),
                points: 8,
                capex_eur: 12_000.0,
            });
        }
    }

    match (input.roof_year, input.roof_material.as_deref(), age) {
        (Some(ry), Some(material), _) if current_year - ry > roof_lifespan(material) => {
            flags.push(RiskFlag {
                label: "Roof past its expected lifespan".into(),
                points: 10,
                capex_eur: 18_000.0,
            });
        }
        (None, _, Some(a)) if a > 35 => {
            flags.push(RiskFlag {
                label: "Roof age unknown on an aging house".into(),
                points: 5,
                capex_eur: 0.0,
            });
        }
        _ => {}
    }

    if input.heating.as_deref().map(|h| h.to_lowercase().contains("oljy") || h.to_lowercase().contains("öljy")).unwrap_or(false) {
        flags.push(RiskFlag {
            label: "Oil heating — phase-out + high running cost (conversion advised)".into(),
            points: 8,
            capex_eur: 0.0,
        });
    }

    if let Some(sewer) = input.sewer_system.as_deref() {
        let s = sewer.to_lowercase();
        let basic = s.contains("saostus") || s.contains("umpis");
        if basic && input.near_water_or_groundwater {
            flags.push(RiskFlag {
                label: "Jätevesi upgrade likely (basic system near water / groundwater)".into(),
                points: 12,
                capex_eur: 12_000.0,
            });
        } else if basic {
            flags.push(RiskFlag {
                label: "Basic jätevesi system — verify asetus 157/2017 compliance".into(),
                points: 5,
                capex_eur: 6_000.0,
            });
        }
    }

    if let Some(cond) = input.condition_class.as_deref() {
        let c = cond.to_lowercase();
        if c.contains("huono") {
            flags.push(RiskFlag { label: "Condition rated poor (huono)".into(), points: 15, capex_eur: 0.0 });
        } else if c.contains("vältt") || c.contains("valtt") {
            flags.push(RiskFlag { label: "Condition rated fair (välttävä)".into(), points: 8, capex_eur: 0.0 });
        }
    }

    if !input.inspection_done {
        flags.push(RiskFlag {
            label: "No condition inspection (kuntotarkastus) on record".into(),
            points: 6,
            capex_eur: 0.0,
        });
    }

    for token in &input.risk_structures {
        let t = token.to_lowercase();
        if t.contains("valesokkeli") || t.contains("salaoj") {
            continue;
        }
        flags.push(RiskFlag {
            label: format!("Risk structure: {token}"),
            points: 10,
            capex_eur: 10_000.0,
        });
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
        let input = RiskInput {
            build_year: Some(1980),
            inspection_done: true,
            ..Default::default()
        };
        let a = assess(&input, 2026);
        assert!(a.flags.iter().any(|f| f.label.contains("Putkiremontti")));
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

    /// A *known* renewal that is itself decades old must still flag overdue — the old
    /// `is_some()` check let any renovation year, however ancient, clear the flag.
    #[test]
    fn old_known_pipe_renewal_still_flags_overdue() {
        let input = RiskInput {
            build_year: Some(1965),
            pipes_renovated_year: Some(1982),
            inspection_done: true,
            ..Default::default()
        };
        let a = assess(&input, 2026);
        let pipe = a.flags.iter().find(|f| f.label.contains("Putkiremontti")).expect("pipe flag");
        assert!(pipe.label.contains("overdue"), "got {:?}", pipe.label);
        assert_eq!(pipe.points, 18);
    }

    /// The user's complaint: an UNKNOWN pipe age must not be sold as a confident
    /// €30k known defect. It's scored lighter than a known-overdue and labelled to
    /// verify — risk reflects uncertainty, not a fabricated certainty.
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
        let kp = known_overdue.flags.iter().find(|f| f.label.contains("Putkiremontti")).unwrap();
        let up = unknown.flags.iter().find(|f| f.label.contains("Putkiremontti")).unwrap();
        assert!(up.points < kp.points, "unknown ({}) must score below known-overdue ({})", up.points, kp.points);
        assert!(up.label.contains("verify"), "unknown flag must read as a verify item: {:?}", up.label);
    }

    /// Renewal stated in the prose (but undated) suppresses the "original pipes"
    /// assumption — the model has evidence, just not a year.
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
        assert!(a.flags.iter().all(|f| !f.label.contains("Putkiremontti") || f.capex_eur == 0.0));
    }

    #[test]
    fn jatevesi_near_water_flags_upgrade() {
        let input = RiskInput {
            build_year: Some(2000),
            sewer_system: Some("saostuskaivo".into()),
            near_water_or_groundwater: true,
            inspection_done: true,
            ..Default::default()
        };
        let a = assess(&input, 2026);
        assert!(a.flags.iter().any(|f| f.label.contains("Jätevesi")));
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
}
