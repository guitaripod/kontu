//! Rank listings by fit to the saved [`Spec`]. Hard criteria filter; soft criteria
//! score. The total-cost-of-ownership term (via the local cost engine) is the
//! dominant weight, matching the user's "cost as close to zero as possible".
//! Soft signals come from structured fields + Finnish description keywords; the
//! research pass refines the keyword lists and adds geo (distance-to-water).

use serde::Serialize;

use crate::app::CostState;
use crate::cost::CostDefaults;
use crate::models::Listing;
use crate::risk;
use crate::spec::{Pref, Spec};

#[derive(Debug, Clone, Serialize)]
pub struct Scored {
    pub id: i64,
    pub title: String,
    pub municipality: Option<String>,
    pub price_eur: Option<i64>,
    pub property_type: Option<String>,
    pub url: String,
    pub score: f64,
    pub npv_cost: f64,
    pub monthly: f64,
    /// Year-1 out-of-pocket running cost (heating, taxes, upkeep, insurance +
    /// any loan interest), excluding equity-building principal: the "cost of living".
    pub monthly_living: f64,
    pub risk: u32,
    pub reasons: Vec<String>,
}

struct Signals {
    shore: f64,
    privacy: f64,
    ev: f64,
    fiber: f64,
    infra: f64,
    winter: f64,
    condition: f64,
}

fn has(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| text.contains(n))
}

/// Lowercase + strip Finnish diacritics, mirroring the Worker's `asciiFold` so a
/// spec `--type mökki` matches a listing stored (folded) as `mokki`.
fn fold_ascii(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'ä' | 'å' | 'Ä' | 'Å' => 'a',
            'ö' | 'Ö' => 'o',
            other => other.to_ascii_lowercase(),
        })
        .collect()
}

fn shore_signal(l: &Listing, desc: &str) -> f64 {
    let structured: f64 = match l.shore.as_deref() {
        Some(s) if s.contains("oma_ranta") => 1.0,
        Some(s) if s.contains("rantaoik") => 0.7,
        Some(s) if s.contains("ei_ranta") => 0.0,
        _ => -1.0,
    };
    let textual = if has(desc, &["rantasauna", "oma ranta", "omarant"]) {
        0.95
    } else if has(desc, &["ranta", "järv", "jarv", "rannal", "vesist", "mökki jär"]) {
        0.6
    } else {
        0.0
    };
    let base = structured.max(textual).max(0.0);
    // The buyer wants a lake; a river (joki) shore is the same ownership but not it.
    let lake_factor = match l.water_body.as_deref() {
        Some(w) if w.contains("joki") => 0.5,
        Some(w) if w.contains("lampi") => 0.8,
        Some(w) if w.contains("meri") => 0.9,
        _ => 1.0,
    };
    base * lake_factor
}

fn privacy_signal(l: &Listing, desc: &str) -> f64 {
    let plot: f64 = match l.plot_area_m2 {
        Some(p) if p >= 5000.0 => 1.0,
        Some(p) if p >= 2000.0 => 0.75,
        Some(p) if p >= 1000.0 => 0.5,
        Some(_) => 0.3,
        None => 0.3,
    };
    let mut s = plot;
    if has(desc, &["rauhalli", "haja-asutus", "ei naapur", "syrjäss", "luonnonrauha", "metsän", "oma rauha", "näköest", "naapureita ei"]) {
        s = (s + 0.3).min(1.0);
    }
    if has(desc, &["keskust", "taajam", "keskeisel", "palvelut vier", "kerrostal"]) {
        s *= 0.4;
    }
    s
}

fn ev_signal(l: &Listing, desc: &str) -> f64 {
    if has(desc, &["sähköaut", "sahkoaut", "latauspist", "latausval", "ev-lat", "3x25", "3 x 25", "kolmivaih", "3-vaih", "63a", "35a"]) {
        1.0
    } else if has(desc, &["autolämmit", "autoläm", "tolppa", "lämmityspist", "autotalli", "autokatos"]) {
        0.6
    } else if l
        .property_type
        .as_deref()
        .map(|t| t.contains("omakoti") || t.contains("pari") || t.contains("erillis") || t.contains("mökki") || t.contains("mokki"))
        .unwrap_or(false)
    {
        0.4
    } else {
        0.2
    }
}

fn fiber_signal(desc: &str, l: &Listing) -> f64 {
    if has(desc, &["valokuit", "valokaapeli", "kuituyht", "kuituliit"])
        || l.broadband.as_deref().map(|b| b.contains("kuitu")).unwrap_or(false)
    {
        1.0
    } else if has(desc, &["laajakaista", "100m", "1000m"]) {
        0.5
    } else {
        0.0
    }
}

fn infra_signal(l: &Listing, desc: &str) -> f64 {
    if has(desc, &["ei sähkö", "kantovesi", "ei vesijoht", "ei viemär", "ei tieyhte"]) {
        return 0.2;
    }
    let mut s: f64 = if has(desc, &["kunnallistek", "kunnallinen vesi", "vesijohto", "viemäri", "kaupungin vesi"]) {
        1.0
    } else if has(desc, &["porakaivo", "rengaskaivo", "oma kaivo"]) {
        0.8
    } else {
        match l.property_type.as_deref() {
            Some(t) if t.contains("omakoti") || t.contains("pari") || t.contains("rivi") || t.contains("kerros") => 0.7,
            Some(t) if t.contains("mökki") || t.contains("mokki") || t.contains("loma") => 0.4,
            _ => 0.5,
        }
    };
    if l.water_supply.is_some() || l.sewer_system.is_some() {
        s = (s + 0.1).min(1.0);
    }
    s
}

/// Year-round liveability: 1.0 = clearly winterized, ~0.1 = clearly summer-only.
/// Explicit text wins; otherwise a non-leisure house is year-round by construction,
/// while a mökki is inferred from central heating and plumbed (not carried) water.
fn winter_signal(l: &Listing, desc: &str) -> f64 {
    // A plot/fixer sold on the *potential* to build year-round, or for demolition,
    // is not a move-in year-round home ("mahdollisuus rakentaa ympärivuotiseen…").
    if has(desc, &["rakennuspaik", "rakennusoikeu", "rakentaa ympärivuoti", "mahdollisuus rakentaa", "purettav", "purkukunt"]) {
        return 0.15;
    }
    if has(desc, &["ei talviasut", "vain kesä", "kesäkäyt", "kesämök", "kesäasun", "kesahuvila", "ei lämmi"]) {
        return 0.1;
    }
    if has(desc, &["talviasutt", "ympärivuoti", "ympäri vuoden", "talvikäyt", "talviasun"]) {
        return 1.0;
    }
    let is_house = l
        .property_type
        .as_deref()
        .map(|t| {
            t.contains("omakoti")
                || t.contains("pari")
                || t.contains("rivi")
                || t.contains("kerros")
                || t.contains("erillis")
        })
        .unwrap_or(false);
    if is_house {
        return 0.9;
    }
    if has(desc, &["kantovesi", "ei vesijoht", "ei sähkö", "ulkohuussi", "kuivakäymälä"]) {
        return 0.2;
    }
    let real_heat = l
        .heating_type
        .as_deref()
        .map(|h| {
            h.contains("kaukolämp")
                || h.contains("maalämp")
                || h.contains("öljy")
                || h.contains("ivlp")
                || h.contains("ilmavesi")
        })
        .unwrap_or(false);
    if real_heat || has(desc, &["lämmin vesi", "eristett", "talvella"]) {
        return 0.7;
    }
    0.4
}

/// Structural condition: 1.0 = move-in/renovated, ~0.2 = needs major work.
/// Explicit "good condition" / "needs renovation" text wins; otherwise build year
/// drives it (the ~1960–85 valesokkeli/putki era is penalized; newer is better).
fn condition_signal(l: &Listing, desc: &str) -> f64 {
    if has(desc, &["remontin tarp", "remontoitav", "peruskorjauksen tarp", "peruskorjattava", "purkukunt", "purettav", "huonokuntoi", "korjausvel", "kosteusvaur", "homevaur", "asumiskelvot", "rakennuspaik", "rakennusoikeu"]) {
        return 0.2;
    }
    let mut base: f64 = if has(desc, &["muuttovalmi", "hyväkuntoi", "erinomaisessa kun", "erinomainen kun", "täysin remontoi", "remontoitu", "peruskorjattu", "uudisveroi", "hyvin pidet"]) {
        0.95
    } else {
        match l.year_built {
            Some(y) if y >= 2010 => 0.9,
            Some(y) if y >= 1995 => 0.8,
            Some(y) if y >= 1986 => 0.65,
            Some(y) if y >= 1960 => 0.45,
            Some(_) => 0.55,
            None => 0.55,
        }
    };
    if l.condition_class.as_deref().map(|c| c.contains("hyvä") || c.contains("erinomai")).unwrap_or(false) {
        base = base.max(0.85);
    }
    // The buyer wants kuntoluokka hyvä or better, so "tyydyttävä" (satisfactory)
    // and below are kept under the Required threshold and thus hard-dropped.
    if l.condition_class.as_deref().map(|c| c.contains("tyydyttäv")).unwrap_or(false)
        || has(desc, &["tyydyttäväss", "tyydyttävä kun"])
    {
        base = base.min(0.45);
    }
    if l.condition_class.as_deref().map(|c| c.contains("huono") || c.contains("välttäv")).unwrap_or(false) {
        base = base.min(0.35);
    }
    base
}

fn signals(l: &Listing) -> Signals {
    let desc = l
        .description
        .as_deref()
        .map(str::to_lowercase)
        .unwrap_or_default();
    Signals {
        shore: shore_signal(l, &desc),
        privacy: privacy_signal(l, &desc),
        ev: ev_signal(l, &desc),
        fiber: fiber_signal(&desc, l),
        infra: infra_signal(l, &desc),
        winter: winter_signal(l, &desc),
        condition: condition_signal(l, &desc),
    }
}

const PRESENT: f64 = 0.45;

/// Soft signals are unreliable (often only in free text), so a Required pref must
/// not hard-drop on a weak/unknown signal — it boosts the ranking weight instead.
/// Only an explicitly *Avoided* trait that is clearly present drops a listing.
fn pref_excludes(pref: Pref, signal: f64) -> bool {
    matches!(pref, Pref::Avoid) && signal > 0.6
}

/// Ranking weight for a soft criterion, scaled by how much the user cares.
fn pref_weight(pref: Pref, base: f64) -> f64 {
    match pref {
        Pref::Required => base * 2.0,
        Pref::Plus => base,
        Pref::Avoid => base * 0.5,
        Pref::Any => base * 0.3,
    }
}

/// For an avoided trait, reward its ABSENCE (penalize presence); otherwise score
/// presence. Keeps each term in [0,1] so the additive `/ total_w` normalization
/// stays valid (an avoided trait that is present lowers the score, never inflates it).
fn pref_signal(pref: Pref, signal: f64) -> f64 {
    if matches!(pref, Pref::Avoid) {
        1.0 - signal
    } else {
        signal
    }
}

fn passes_hard(spec: &Spec, l: &Listing, s: &Signals) -> bool {
    if let Some(max) = spec.price_max
        && l.price_eur.map(|p| p > max).unwrap_or(true) {
            return false;
        }
    if let Some(min) = spec.price_min
        && l.price_eur.map(|p| p < min).unwrap_or(false) {
            return false;
        }
    if !spec.property_types.is_empty() {
        let t = fold_ascii(l.property_type.as_deref().unwrap_or(""));
        if !spec.property_types.iter().any(|want| t.contains(&fold_ascii(want))) {
            return false;
        }
    }
    if !spec.municipalities.is_empty() {
        let m = l.municipality.as_deref().unwrap_or("");
        if !spec.municipalities.iter().any(|want| want.eq_ignore_ascii_case(m)) {
            return false;
        }
    }
    if let Some(y) = spec.year_min
        && l.year_built.map(|b| b < y).unwrap_or(false) {
            return false;
        }
    if let Some(m) = spec.min_m2
        && l.living_area_m2.map(|a| a < m).unwrap_or(false) {
            return false;
        }
    if let Some(r) = spec.min_rooms
        && l.room_count.map(|c| c < r).unwrap_or(false) {
            return false;
        }
    if let Some(p) = spec.min_plot_m2
        && l.plot_area_m2.map(|a| a < p).unwrap_or(false) {
            return false;
        }
    if let Some(d) = spec.max_dom
        && l.days_on_market.map(|x| x > d).unwrap_or(false) {
            return false;
        }
    if !spec.exclude.is_empty() {
        let hay = format!(
            "{} {} {}",
            l.title(),
            l.description.clone().unwrap_or_default(),
            l.municipality.clone().unwrap_or_default()
        )
        .to_lowercase();
        if spec.exclude.iter().any(|kw| hay.contains(&kw.to_lowercase())) {
            return false;
        }
    }
    if spec.owned_plot
        && l.plot_ownership
            .as_deref()
            .map(|o| o.contains("vuokra"))
            .unwrap_or(false)
    {
        return false;
    }
    if pref_excludes(spec.shore, s.shore)
        || pref_excludes(spec.ev_charging, s.ev)
        || pref_excludes(spec.fiber, s.fiber)
        || pref_excludes(spec.privacy, s.privacy)
        || pref_excludes(spec.winterized, s.winter)
        || pref_excludes(spec.condition, s.condition)
    {
        return false;
    }
    // Unlike the free-text soft signals, a clearly summer-only listing is a real
    // disqualifier for a year-round home, so Required hard-drops it.
    if matches!(spec.winterized, Pref::Required) && s.winter < 0.3 {
        return false;
    }
    // A clearly renovation-needed / renovation-era house defeats "good condition".
    if matches!(spec.condition, Pref::Required) && s.condition < 0.5 {
        return false;
    }
    if spec.require_infra && s.infra < 0.25 {
        return false;
    }
    true
}

struct Candidate {
    listing: Listing,
    signals: Signals,
    npv: f64,
    monthly: f64,
    living: f64,
    risk: u32,
}

/// Rank the listings by fit to the spec, best first.
pub fn rank(spec: &Spec, listings: Vec<Listing>, defaults: &CostDefaults) -> Vec<Scored> {
    let mut candidates: Vec<Candidate> = Vec::new();
    for l in listings {
        let s = signals(&l);
        if !passes_hard(spec, &l, &s) {
            continue;
        }
        let assessment = risk::assess(&l.to_risk_input(s.shore >= PRESENT), 2026);
        let mut cs = CostState::from_defaults(defaults);
        cs.apply_listing(&l, &assessment, defaults);
        cs.horizon = spec.horizon_years;
        if spec.cash {
            cs.ltv = 0.0;
        }
        let proj = cs.project(defaults);
        candidates.push(Candidate {
            listing: l,
            signals: s,
            npv: proj.npv_cost,
            monthly: proj.equivalent_monthly,
            living: proj.years.first().map(|y| (y.recurring + y.interest) / 12.0).unwrap_or(0.0),
            risk: assessment.score,
        });
    }

    let (min_npv, max_npv) = candidates.iter().fold((f64::MAX, f64::MIN), |(lo, hi), c| {
        (lo.min(c.npv), hi.max(c.npv))
    });

    let w = &spec.weights;
    let wtco = if spec.minimize_tco { w.tco * 2.0 } else { w.tco };
    let ws = pref_weight(spec.shore, w.shore);
    let wp = pref_weight(spec.privacy, w.privacy);
    let we = pref_weight(spec.ev_charging, w.ev);
    let wf = pref_weight(spec.fiber, w.fiber);
    let ww = pref_weight(spec.winterized, w.winter);
    let wc = pref_weight(spec.condition, w.condition);
    let wi = if spec.require_infra { w.infra * 1.5 } else { w.infra };
    let total_w = (wtco + ws + wp + we + wf + ww + wc + wi + w.risk).max(1e-9);

    let mut scored: Vec<Scored> = candidates
        .into_iter()
        .map(|c| {
            let tco = if (max_npv - min_npv).abs() < 1.0 {
                0.6
            } else {
                1.0 - (c.npv - min_npv) / (max_npv - min_npv)
            };
            let risk_score = 1.0 - (c.risk as f64 / 100.0);
            let score = (wtco * tco
                + ws * pref_signal(spec.shore, c.signals.shore)
                + wp * pref_signal(spec.privacy, c.signals.privacy)
                + we * pref_signal(spec.ev_charging, c.signals.ev)
                + wf * pref_signal(spec.fiber, c.signals.fiber)
                + ww * pref_signal(spec.winterized, c.signals.winter)
                + wc * pref_signal(spec.condition, c.signals.condition)
                + wi * c.signals.infra
                + w.risk * risk_score)
                / total_w
                * 100.0;
            Scored {
                id: c.listing.id,
                title: c.listing.title(),
                municipality: c.listing.municipality.clone(),
                price_eur: c.listing.price_eur,
                property_type: c.listing.property_type.clone(),
                url: c.listing.url.clone(),
                reasons: reasons(&c, tco),
                score,
                npv_cost: c.npv,
                monthly: c.monthly,
                monthly_living: c.living,
                risk: c.risk,
            }
        })
        .collect();

    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    scored
}

fn reasons(c: &Candidate, tco: f64) -> Vec<String> {
    let mut r = Vec::new();
    if tco >= 0.66 {
        r.push("low cost of ownership".into());
    }
    if c.signals.shore >= 0.6 {
        r.push("lakeshore".into());
    }
    if c.signals.privacy >= 0.7 {
        if let Some(p) = c.listing.plot_area_m2 {
            r.push(format!("private ({p:.0} m² plot)"));
        } else {
            r.push("private".into());
        }
    }
    if c.signals.winter >= 0.9 {
        r.push("year-round".into());
    } else if c.signals.winter <= 0.2 {
        r.push("summer-only?".into());
    }
    if c.signals.condition >= 0.9 {
        r.push("good condition".into());
    } else if c.signals.condition <= 0.3 {
        r.push("needs work?".into());
    }
    if c.signals.fiber >= 1.0 {
        r.push("fibre".into());
    }
    if c.signals.ev >= 1.0 {
        r.push("EV charging".into());
    } else if c.signals.ev >= 0.6 {
        r.push("car heating point (EV-able)".into());
    }
    if c.risk < 25 {
        r.push("low risk".into());
    }
    if let Some(f) = &c.listing.fairness
        && matches!(f.band.as_str(), "underpriced" | "below_market") {
            r.push("below-market price".into());
        }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_matches_diacritic_spelling_to_stored_ascii() {
        assert_eq!(fold_ascii("mökki"), "mokki");
        assert_eq!(fold_ascii("Mökki"), "mokki");
        assert_eq!(fold_ascii("omakotitalo"), "omakotitalo");
        assert!(fold_ascii("mokki").contains(&fold_ascii("mökki")));
    }

    #[test]
    fn avoided_trait_present_scores_below_absent() {
        assert_eq!(pref_signal(Pref::Avoid, 1.0), 0.0);
        assert_eq!(pref_signal(Pref::Avoid, 0.0), 1.0);
        assert_eq!(pref_signal(Pref::Required, 1.0), 1.0);
    }

    #[test]
    fn winter_signal_separates_year_round_from_summer() {
        let ws = |l: &Listing| winter_signal(l, &l.description.clone().unwrap_or_default().to_lowercase());
        let summer = Listing {
            description: Some("Ihana kesämökki järven rannalla, kantovesi ja puulämmitys.".into()),
            property_type: Some("mökki".into()),
            ..Default::default()
        };
        let year_round = Listing {
            description: Some("Talviasuttava mökki, maalämpö ja kunnallinen vesi.".into()),
            property_type: Some("mökki".into()),
            ..Default::default()
        };
        let house = Listing { property_type: Some("omakotitalo".into()), ..Default::default() };
        assert!(ws(&summer) <= 0.2, "kesämökki + kantovesi reads summer-only");
        assert!(ws(&year_round) >= 0.9, "talviasuttava reads year-round");
        assert!(ws(&house) >= 0.9, "a detached house is year-round by construction");
    }

    #[test]
    fn river_shore_scores_below_lake_shore() {
        let lake = Listing {
            shore: Some("oma_ranta".into()),
            water_body: None,
            ..Default::default()
        };
        let river = Listing {
            shore: Some("oma_ranta".into()),
            water_body: Some("joki".into()),
            ..Default::default()
        };
        assert!(shore_signal(&river, "") < shore_signal(&lake, ""));
        // an unknown water body must NOT be penalized
        assert_eq!(shore_signal(&lake, ""), 1.0);
    }

    #[test]
    fn buildable_plot_is_not_year_round_or_good_condition() {
        let plot = Listing { property_type: Some("omakotitalo".into()), ..Default::default() };
        let desc = "mahdollisuus rakentaa ympärivuotiseen käyttöön, rakennusoikeutta jäljellä";
        assert!(winter_signal(&plot, desc) < 0.3, "build-for-year-round must not read year-round");
        assert!(condition_signal(&plot, desc) <= 0.3, "a buildable plot is not good condition");
    }

    #[test]
    fn condition_signal_reads_text_and_era() {
        let cs = |l: &Listing| condition_signal(l, &l.description.clone().unwrap_or_default().to_lowercase());
        let fixer = Listing {
            description: Some("Vanha talo, remontin tarpeessa, peruskorjaus edessä.".into()),
            ..Default::default()
        };
        let move_in = Listing {
            description: Some("Muuttovalmis, täysin remontoitu, hyväkuntoinen.".into()),
            ..Default::default()
        };
        let valesokkeli_era = Listing { year_built: Some(1975), ..Default::default() };
        let modern = Listing { year_built: Some(2015), ..Default::default() };
        assert!(cs(&fixer) <= 0.3, "remontin tarpeessa reads needs-work");
        assert!(cs(&move_in) >= 0.9, "muuttovalmis/remontoitu reads good condition");
        assert!(cs(&valesokkeli_era) < cs(&modern), "1975 era is riskier than 2015");
    }
}
