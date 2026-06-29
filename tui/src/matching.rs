//! Rank listings by fit to the saved [`Spec`]. Hard criteria filter; soft criteria
//! score. The total-cost-of-ownership term (via the local cost engine) is the
//! dominant weight, matching the user's "cost as close to zero as possible".
//! Soft signals come from structured fields + Finnish description keywords; the
//! research pass refines the keyword lists and adds geo (distance-to-water).

use serde::Serialize;

use crate::cost::CostState;
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
    /// Manually pinned into the options despite failing the gate (`spec.pinned`).
    pub pinned: bool,
    /// Passes every hard criterion but sits above `max_risk` (within the
    /// `near_miss_risk` band): a sound home that just misses on age-risk.
    pub near_miss: bool,
    /// Off-spec on a required *preference* (e.g. no shore) yet priced as a clear
    /// steal vs the area benchmark and structurally sound — surfaced in a separate
    /// "value outliers" lane, never alerted on, never counted as a gate match.
    pub value_outlier: bool,
    /// For a `value_outlier`, the required preferences it fails (Finnish, for the
    /// website) so the card can say exactly why it is off-spec.
    pub off_spec: Vec<String>,
    /// Asking price ÷ area benchmark, when known — the depth of the discount. Drives
    /// the value-outliers lane's ordering and cap (steepest steal first).
    pub fairness_ratio: Option<f64>,
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
            'ä' | 'å' | 'Ä' | 'Å' | 'á' | 'à' | 'Á' => 'a',
            'ö' | 'Ö' | 'ó' | 'ø' | 'Ø' | 'Ó' | 'ò' => 'o',
            'í' | 'ì' | 'Í' => 'i',
            'ý' | 'Ý' => 'y',
            'ú' | 'ü' | 'Ú' | 'ù' => 'u',
            'é' | 'è' | 'É' => 'e',
            'ð' | 'Ð' => 'd',
            other => other.to_ascii_lowercase(),
        })
        .collect()
}

/// Canonical cross-Nordic property family, so a Finnish `omakotitalo`, Swedish
/// `villa`, Norwegian `enebolig`, Danish `detached` and Icelandic `einbýlishús`
/// all classify the same. Lets a spec written in Finnish tokens match listings
/// normalized to any country's vocabulary.
fn property_family(token: &str) -> &'static str {
    let t = fold_ascii(token);
    // Leisure is checked first: "holiday house" / "fritidshus" must not be caught
    // by the "house" branch.
    if has(&t, &["mokki", "loma", "vapaa-ajan", "fritid", "leisure", "holiday", "hytte", "sumarhus", "sumarbustad", "cottage"]) {
        "leisure"
    } else if has(&t, &["omakoti", "erillis", "detached", "einbyli", "enebolig", "villa", "parcelhus", "fritliggende", "house"]) {
        "house"
    } else if has(&t, &["pari", "tomanns", "parhus", "kedjehus", "dobbelthus", "semi"]) {
        "semi"
    } else if has(&t, &["rivi", "terraced", "radhus", "raekkehus", "rekkehus", "kaede"]) {
        "terraced"
    } else if has(&t, &["kerrostal", "apartment", "lagenhet", "leilighet", "haed", "condo", "ibud", "ejerlejlighed", "fjolbyli"]) {
        "apartment"
    } else if has(&t, &["maatila", "farm", "gard", "smabruk", "jord", "landejendom"]) {
        "farm"
    } else if has(&t, &["plot", "tomt", "land", "mark", "tontti", "maaraala", "lod"]) {
        "plot"
    } else {
        "other"
    }
}

/// True when the listing's shore field (in any country's normalized vocabulary)
/// denotes OWNING the waterfront.
fn shore_is_own(shore: &str) -> bool {
    let s = shore.to_lowercase();
    has(&s, &["oma_ranta", "own_shore", "egen strand", "sjotomt", "strandtomt", "sjavarlod", "sjotomt"])
}

fn shore_signal(l: &Listing, desc: &str) -> f64 {
    let structured: f64 = match l.shore.as_deref().map(|s| s.to_lowercase()) {
        Some(ref s) if shore_is_own(s) => 1.0,
        Some(ref s) if has(s, &["rantaoik", "shore_right", "strandratt", "strandrett"]) => 0.7,
        Some(ref s) if has(s, &["water_view", "sea_view", "sjoutsikt", "havudsigt", "soeudsigt"]) => 0.3,
        Some(ref s) if has(s, &["ei_ranta", "no_shore"]) => 0.0,
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
/// A bare buildable plot / teardown, not a move-in home: explicit demolition, or
/// "build here" language on a listing with no real dwelling. NOT triggered by
/// `rakennusoikeutta jäljellä` (spare building rights) — that's a feature of an
/// existing home, common on the spacious lakeside plots the buyer wants.
fn is_buildable_plot(l: &Listing, desc: &str) -> bool {
    if has(desc, &["purettav", "purkukunt"]) {
        return true;
    }
    // Sold AS a build site — a permit/right to build a new cabin — even when a
    // spurious "Asuinpinta-ala" suggests a dwelling (only a shed exists). Distinct
    // from "rakennusoikeutta jäljellä" = spare building rights on an existing home.
    if has(desc, &["poikkeamislupa", "rakennuspaik", "rakentamiselle", "rakentaa vapaa", "rakentaa loma", "rakennettavaksi"])
        || (has(desc, &["rakennusoikeus"]) && has(desc, &["loma-asun", "lomaasun", "vapaa-ajan asun"]))
    {
        return true;
    }
    // Weaker "build here" hints only count when there's no real dwelling.
    let no_dwelling = l.living_area_m2.map(|a| a < 20.0).unwrap_or(true);
    no_dwelling && has(desc, &["mahdollisuus rakentaa", "rakentaa ympärivuoti", "rakennusmahdollisuu"])
}

/// Clearly more than one living floor — for the single-storey hard filter.
/// `kellari` (basement) alone does NOT count; only explicit multi-level wording or
/// an upstairs/downstairs (yläkerta/alakerta) does.
fn is_multi_floor(desc: &str) -> bool {
    has(
        desc,
        &[
            "kaksikerroksi", "kaksitasoi", "kahdessa tasossa", "kahteen tasoon",
            "kolmikerroksi", "kolmessa kerroksessa", "puolitoistakerroksi",
            "1,5-kerroksi", "1,5 kerroksi", "1.5-kerroksi",
            "yläkerta", "yläkerrassa", "yläkerran", "yläkertaan", "yläkerroksen",
            "alakerrassa", "alakerran", "molemmista kerroksista", "molemmissa kerroksissa",
            "ylä- ja alakerr", "vinttihuone",
        ],
    )
}

/// Year-round liveability: 1.0 = clearly winterized, ~0.1 = clearly summer-only.
/// An explicit conversion to / statement of permanent residential use (e.g.
/// "rakennettu kesämökiksi, asuinkäyttöön muutettu") overrides a historical
/// "built as a summer cabin" mention — origin is not current use.
fn winter_signal(l: &Listing, desc: &str) -> f64 {
    if is_buildable_plot(l, desc) {
        return 0.15;
    }
    let year_round = has(desc, &[
        "talviasutt", "ympärivuoti", "ympäri vuoden", "talvikäyt", "talviasun",
        "asuinkäyttöön muut", "vakituiseksi muut", "muutettu vakituise", "muutettu asuinkäyt",
    ]);
    if !year_round
        && has(desc, &["ei talviasut", "vain kesä", "kesäkäyt", "kesämök", "kesäasun", "kesahuvila", "ei lämmi"])
    {
        return 0.1;
    }
    if year_round {
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
    if is_buildable_plot(l, desc) {
        return 0.2;
    }
    if has(desc, &["remontin tarp", "remontoitav", "peruskorjauksen tarp", "peruskorjattava", "huonokuntoi", "korjausvel", "kosteusvaur", "homevaur", "asumiskelvot"]) {
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

/// `shore = required` means an OWN LAKE (järvi) shore: own-shore on an unknown
/// water body counts (rural lake listings name the lake only in free text), but
/// river / pond / sea / no-shore do not.
fn own_lake_shore(l: &Listing) -> bool {
    l.shore.as_deref().map(shore_is_own).unwrap_or(false)
        && l
            .water_body
            .as_deref()
            .map(|w| {
                let w = w.to_lowercase();
                !(w.contains("joki") || w.contains("lampi") || w.contains("meri"))
            })
            .unwrap_or(true)
}

/// The non-negotiable filters: budget, type, place, size, plot ownership, single
/// floor, exclusions. A value outlier must clear ALL of these — it may only fall
/// short on a lifestyle *preference* (handled by [`passes_preferences`]).
fn passes_structural(spec: &Spec, l: &Listing) -> bool {
    if let Some(max) = spec.price_max
        && l.price_eur.map(|p| p > max).unwrap_or(true) {
            return false;
        }
    if let Some(min) = spec.price_min
        && l.price_eur.map(|p| p < min).unwrap_or(false) {
            return false;
        }
    if !spec.property_types.is_empty() {
        // Match on the canonical family (so a Finnish-token spec matches Swedish /
        // Danish / Icelandic listings), keeping a substring check as a fallback.
        let t = fold_ascii(l.property_type.as_deref().unwrap_or(""));
        let got = property_family(&t);
        let family_match = spec.property_types.iter().any(|w| property_family(w) == got);
        let substring_match = spec.property_types.iter().any(|w| t.contains(&fold_ascii(w)));
        if got == "other" || (!family_match && !substring_match) {
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
    if spec.single_floor {
        let desc = l.description.as_deref().unwrap_or("").to_lowercase();
        if is_multi_floor(&desc) {
            return false;
        }
    }
    true
}

/// The lifestyle preferences a gate listing must also satisfy but a value outlier
/// is allowed to miss: shore, winter-readiness, condition, basic infra, and any
/// explicitly avoided trait that is clearly present.
fn passes_preferences(spec: &Spec, l: &Listing, s: &Signals) -> bool {
    // Iceland has essentially no lakes, so a lake shore is not required there — a
    // good house anywhere in Iceland qualifies on the shore dimension.
    let shore_applies = lake_country(l);
    if (shore_applies && pref_excludes(spec.shore, s.shore))
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
    if shore_applies && matches!(spec.shore, Pref::Required) && !own_lake_shore(l) {
        return false;
    }
    if spec.require_infra && s.infra < 0.25 {
        return false;
    }
    true
}

/// Whether a lake shore is a meaningful requirement in this listing's country.
/// Iceland (volcanic, near-lakeless) is exempt so its houses aren't all dropped.
fn lake_country(l: &Listing) -> bool {
    l.country_enum() != crate::country::Country::Is
}

/// The required preferences a listing fails, as short Finnish labels for the website.
/// Empty for a clean gate-passer; non-empty is exactly why a value outlier is off-spec.
fn off_spec_reasons(spec: &Spec, l: &Listing, s: &Signals) -> Vec<String> {
    let mut v = Vec::new();
    if lake_country(l) && matches!(spec.shore, Pref::Required) && !own_lake_shore(l) {
        v.push("Ei omaa järvenrantaa".to_string());
    }
    // A detached house on its own plot (ev ≥ 0.4) can always add a charger, so only
    // a genuinely un-chargeable site (apartment-like, no plot/garage) is off-spec.
    if matches!(spec.ev_charging, Pref::Required) && s.ev < 0.4 {
        v.push("Ei sähköauton latausmahdollisuutta".to_string());
    }
    if matches!(spec.privacy, Pref::Required) && s.privacy < 0.6 {
        v.push("Yksityisyyttä ei vahvistettu".to_string());
    }
    if matches!(spec.condition, Pref::Required) && s.condition < 0.5 {
        v.push("Kunto vaatii tarkistusta".to_string());
    }
    if matches!(spec.fiber, Pref::Required) && s.fiber < 0.5 {
        v.push("Ei vahvistettua valokuitua".to_string());
    }
    if spec.require_infra && s.infra < 0.25 {
        v.push("Perusinfra epävarma".to_string());
    }
    v
}

/// A genuine steal that just misses the lifestyle wishes: priced clearly under the
/// area benchmark, structurally sound (not a teardown/summer shack, risk within the
/// near-miss ceiling). Off-spec status is decided by the caller (passes_structural
/// && !passes_preferences); this judges whether the steal is worth surfacing at all.
fn is_value_outlier(spec: &Spec, l: &Listing, s: &Signals, risk: u32) -> bool {
    // A *believable* exceptional discount: 30–80 % under the area benchmark. A ratio
    // under 0.2 (>80 % "off") is almost always bad data — a price-on-request
    // placeholder, a property share, or a benchmark that doesn't fit a tiny
    // micro-area — not a real steal, so it must not headline the lane.
    const MIN_RATIO: f64 = 0.2;
    const MAX_RATIO: f64 = 0.7;
    let steal = l
        .fairness
        .as_ref()
        .and_then(|f| f.ratio)
        .map(|r| (MIN_RATIO..MAX_RATIO).contains(&r))
        .unwrap_or(false);
    let desc = l.description.as_deref().unwrap_or("");
    // A real, evaluable home — not a near-empty placeholder row or a shed/plot.
    let real_home = l.living_area_m2.map(|a| a >= 30.0).unwrap_or(false) && desc.len() >= 40;
    let sound = !is_buildable_plot(l, &desc.to_lowercase()) && s.condition >= 0.4 && s.winter >= 0.3;
    let risk_ok = risk <= spec.near_miss_risk.unwrap_or(50);
    // If it's giving up the lake shore, it must earn the spot another way: seclusion.
    // A house with no own lake AND unconfirmed privacy (close neighbours) is not worth
    // surfacing. (Low bug pressure is the other half, enforced on the Worker which owns
    // the geodata bug signal.)
    let secluded = own_lake_shore(l) || s.privacy >= 0.6;
    steal && real_home && sound && risk_ok && secluded
}

/// Confirmed move-in soundness from the structured kuntoluokka. The near-miss band
/// requires it — the band is "a sound home that's merely older", so era-inferred or
/// unknown-condition listings must not leak into the safety net.
fn confirmed_sound(l: &Listing) -> bool {
    l.condition_class
        .as_deref()
        .map(|c| {
            let c = c.to_lowercase();
            c.contains("hyvä") || c.contains("erinomai")
        })
        .unwrap_or(false)
}

struct Candidate {
    listing: Listing,
    signals: Signals,
    npv: f64,
    monthly: f64,
    living: f64,
    risk: u32,
    pinned: bool,
    near_miss: bool,
    value_outlier: bool,
    off_spec: Vec<String>,
}

/// Rank the spec's quality-gate survivors by fit, best first. The primary set is
/// the gate-passing set: listings that cleared the hard gate (`passes_structural`
/// && `passes_preferences`) *and* the `max_risk`
/// cap (membership is binary, decided solely by the gate — fit only ORDERS it, so
/// the watch alerts on it directly and must not re-gate on the relative fit). Two
/// marked extras ride alongside: `near_miss` homes that pass every hard criterion
/// but sit in the `(max_risk, near_miss_risk]` band (sound, just older), and
/// `pinned` homes the user force-includes. Neither distorts the gate set's fit.
pub fn rank(spec: &Spec, listings: Vec<Listing>, defaults: &CostDefaults) -> Vec<Scored> {
    let mut candidates: Vec<Candidate> = Vec::new();
    for l in listings {
        // Country scope: when the spec names countries, only those are in play
        // (pins still ride along, like the gate).
        if !spec.countries.is_empty()
            && !spec.pinned.contains(&l.id)
            && !spec.countries.iter().any(|c| c.eq_ignore_ascii_case(l.country_enum().code()))
        {
            continue;
        }
        let s = signals(&l);
        let pinned = spec.pinned.contains(&l.id);
        let passes_struct = passes_structural(spec, &l);
        let passes = passes_struct && passes_preferences(spec, &l, &s);
        // A value outlier clears every structural filter but misses a lifestyle
        // preference; gate the cheap fairness pre-check before paying for risk.
        let outlier_eligible = !passes
            && passes_struct
            && l.fairness
                .as_ref()
                .map(|f| matches!(f.band.as_str(), "underpriced" | "below_market"))
                .unwrap_or(false);
        if !passes && !pinned && !outlier_eligible {
            continue;
        }
        let assessment = risk::assess(&l.to_risk_input(s.shore >= PRESENT), 2026);
        let within_gate = passes && spec.max_risk.map(|m| assessment.score <= m).unwrap_or(true);
        let within_near = passes
            && !within_gate
            && confirmed_sound(&l)
            && spec.near_miss_risk.map(|n| assessment.score <= n).unwrap_or(false);
        let candidate_outlier = outlier_eligible
            && !pinned
            && !within_gate
            && !within_near
            && is_value_outlier(spec, &l, &s, assessment.score);
        let off_spec = if candidate_outlier {
            off_spec_reasons(spec, &l, &s)
        } else {
            Vec::new()
        };
        // A value outlier must be able to say WHY it's off-spec; if the only failing
        // preference is an Avoided-trait-present (no positive reason to display), don't
        // surface it with a blank justification.
        let value_outlier = candidate_outlier && !off_spec.is_empty();
        if !pinned && !within_gate && !within_near && !value_outlier {
            continue;
        }
        let cd = CostDefaults::resolve(defaults, l.country_enum());
        let mut cs = CostState::from_defaults(&cd);
        cs.apply_listing(&l, &assessment, &cd);
        cs.horizon = spec.horizon_years;
        if spec.cash {
            cs.ltv = 0.0;
        }
        let proj = cs.project(&cd);
        candidates.push(Candidate {
            listing: l,
            signals: s,
            npv: proj.npv_cost,
            monthly: proj.equivalent_monthly,
            living: proj.years.first().map(|y| (y.recurring + y.interest) / 12.0).unwrap_or(0.0),
            risk: assessment.score,
            pinned,
            near_miss: within_near && !pinned,
            value_outlier,
            off_spec,
        });
    }

    let gate_only = candidates
        .iter()
        .filter(|c| !c.near_miss && !c.pinned && !c.value_outlier);
    let (mut min_npv, mut max_npv) = gate_only.fold((f64::MAX, f64::MIN), |(lo, hi), c| {
        (lo.min(c.npv), hi.max(c.npv))
    });
    if min_npv > max_npv {
        let (lo, hi) = candidates.iter().fold((f64::MAX, f64::MIN), |(lo, hi), c| {
            (lo.min(c.npv), hi.max(c.npv))
        });
        min_npv = lo;
        max_npv = hi;
    }

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
                pinned: c.pinned,
                near_miss: c.near_miss,
                value_outlier: c.value_outlier,
                off_spec: c.off_spec.clone(),
                fairness_ratio: c.listing.fairness.as_ref().and_then(|f| f.ratio),
            }
        })
        .collect();

    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    scored
}

fn reasons(c: &Candidate, tco: f64) -> Vec<String> {
    let mut r = Vec::new();
    if c.pinned {
        r.push("📌 pinned".into());
    } else if c.near_miss {
        r.push(format!("near-miss · risk {}", c.risk));
    } else if c.value_outlier {
        r.push("value outlier — below market, off-spec".into());
    }
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
    fn converted_summer_cabin_reads_year_round() {
        let converted = Listing {
            property_type: Some("omakotitalo".into()),
            description: Some("2000 rakennettu kesämökiksi, asuinkäyttöön muutettu 2006. Kunto hyvä.".into()),
            ..Default::default()
        };
        let ws = winter_signal(&converted, &converted.description.clone().unwrap().to_lowercase());
        assert!(ws >= 0.9, "a cabin converted to year-round use is not summer-only (got {ws})");
    }

    #[test]
    fn near_miss_band_and_pins_ride_alongside_the_gate() {
        let defaults = CostDefaults::default();
        let mk = |id: i64, year: i32| Listing {
            id,
            shore: Some("oma_ranta".into()),
            property_type: Some("omakotitalo".into()),
            condition_class: Some("hyvä".into()),
            price_eur: Some(90_000),
            year_built: Some(year),
            ..Default::default()
        };
        let teardown = |id| Listing { description: Some("purkukuntoinen mökki".into()), ..mk(id, 1950) };
        let spec = Spec {
            shore: Pref::Required,
            winterized: Pref::Required,
            condition: Pref::Required,
            max_risk: Some(25),
            near_miss_risk: Some(50),
            pinned: vec![3],
            cash: true,
            ..Default::default()
        };
        let scored = rank(&spec, vec![mk(1, 2010), mk(2, 1980), teardown(3), teardown(4)], &defaults);
        let find = |id| scored.iter().find(|s| s.id == id);
        assert!(matches!(find(1), Some(s) if !s.near_miss && !s.pinned), "newish low-risk house is a clean gate-passer");
        assert!(matches!(find(2), Some(s) if s.near_miss && !s.pinned), "sound 1980 own-lake house is a near-miss, not dropped");
        assert!(matches!(find(3), Some(s) if s.pinned), "pinned teardown is force-included");
        assert!(find(4).is_none(), "an unpinned teardown stays out of the gate");
    }

    #[test]
    fn underpriced_off_spec_home_surfaces_as_a_value_outlier() {
        let defaults = CostDefaults::default();
        let steal = |band: &str| Listing {
            id: 10,
            property_type: Some("omakotitalo".into()),
            condition_class: Some("hyvä".into()),
            price_eur: Some(39_000),
            year_built: Some(1970),
            plot_ownership: Some("oma".into()),
            plot_area_m2: Some(2000.0),
            living_area_m2: Some(100.0),
            description: Some("Hyväkuntoinen omakotitalo rauhallisella paikalla, iso oma tontti.".into()),
            shore: None,
            fairness: Some(crate::models::Fairness {
                band: band.into(),
                ratio: Some(0.34),
                benchmark: Some(115_000.0),
                confidence: "medium".into(),
            }),
            ..Default::default()
        };
        let spec = Spec {
            shore: Pref::Required,
            condition: Pref::Required,
            winterized: Pref::Required,
            single_floor: true,
            max_risk: Some(25),
            near_miss_risk: Some(50),
            price_max: Some(100_000),
            cash: true,
            ..Default::default()
        };

        let scored = rank(&spec, vec![steal("underpriced")], &defaults);
        let s = scored.iter().find(|s| s.id == 10).expect("the steal must be surfaced");
        assert!(s.value_outlier && !s.near_miss && !s.pinned, "no-shore underpriced sound home is a value outlier");
        assert!(
            s.off_spec.iter().any(|r| r.contains("rantaa")),
            "off-spec must name the missed shore requirement: {:?}",
            s.off_spec
        );

        // A fairly-priced off-spec home is NOT a steal — the lane only surfaces genuine value.
        assert!(
            rank(&spec, vec![steal("fair")], &defaults).iter().find(|s| s.id == 10).is_none(),
            "a fair-priced no-shore home must not enter the outlier lane"
        );
    }

    #[test]
    fn outlier_lane_rejects_teardowns_and_over_budget_steals() {
        let defaults = CostDefaults::default();
        let underpriced = || Some(crate::models::Fairness {
            band: "underpriced".into(),
            ratio: Some(0.3),
            benchmark: Some(115_000.0),
            confidence: "medium".into(),
        });
        let teardown = Listing {
            id: 20,
            property_type: Some("omakotitalo".into()),
            description: Some("Purkukuntoinen mökki, ei rantaa.".into()),
            price_eur: Some(25_000),
            plot_ownership: Some("oma".into()),
            shore: None,
            fairness: underpriced(),
            ..Default::default()
        };
        let over_budget = Listing {
            id: 21,
            property_type: Some("omakotitalo".into()),
            condition_class: Some("hyvä".into()),
            price_eur: Some(150_000),
            year_built: Some(1975),
            plot_ownership: Some("oma".into()),
            shore: None,
            fairness: underpriced(),
            ..Default::default()
        };
        // A no-shore steal hemmed in by neighbours (tiny in-town plot) earns no place:
        // it gave up the lake AND offers no seclusion in return.
        let neighbours = Listing {
            id: 22,
            property_type: Some("omakotitalo".into()),
            condition_class: Some("hyvä".into()),
            price_eur: Some(45_000),
            year_built: Some(1975),
            plot_ownership: Some("oma".into()),
            plot_area_m2: Some(500.0),
            living_area_m2: Some(90.0),
            description: Some("Siisti omakotitalo aivan keskustan tuntumassa, palvelut vieressä.".into()),
            shore: None,
            fairness: underpriced(),
            ..Default::default()
        };
        let spec = Spec {
            shore: Pref::Required,
            condition: Pref::Required,
            single_floor: true,
            max_risk: Some(25),
            near_miss_risk: Some(50),
            price_max: Some(100_000),
            cash: true,
            ..Default::default()
        };
        let scored = rank(&spec, vec![teardown, over_budget, neighbours], &defaults);
        assert!(scored.iter().find(|s| s.id == 20).is_none(), "a cheap teardown is not a value outlier");
        assert!(scored.iter().find(|s| s.id == 21).is_none(), "an over-budget steal fails the structural gate");
        assert!(scored.iter().find(|s| s.id == 22).is_none(), "a no-shore steal with close neighbours is not surfaced");
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
        let plot = Listing {
            property_type: Some("omakotitalo".into()),
            living_area_m2: None,
            ..Default::default()
        };
        let desc = "rakennuspaikka järven rannalla, mahdollisuus rakentaa ympärivuotiseen";
        assert!(winter_signal(&plot, desc) < 0.3, "a bare buildable plot must not read year-round");
        assert!(condition_signal(&plot, desc) <= 0.3, "a buildable plot is not good condition");
        // A build permit must be caught even when a spurious Asuinpinta-ala is set.
        let permit = Listing {
            living_area_m2: Some(63.0),
            description: Some("Metsäkiinteistö jossa poikkeamislupa vapaa-ajan asunnon rakentamiselle.".into()),
            ..Default::default()
        };
        assert!(is_buildable_plot(&permit, "poikkeamislupa vapaa-ajan asunnon rakentamiselle"));
    }

    #[test]
    fn real_home_with_spare_building_rights_is_not_dropped() {
        // "rakennusoikeutta jäljellä" = spare building rights, a FEATURE of a real home.
        let home = Listing {
            property_type: Some("omakotitalo".into()),
            living_area_m2: Some(95.0),
            year_built: Some(2005),
            description: Some("Hyväkuntoinen talo, rakennusoikeutta jäljellä 120 m².".into()),
            ..Default::default()
        };
        assert!(!is_buildable_plot(&home, "rakennusoikeutta jäljellä 120 m²"));
        assert!(condition_signal(&home, "hyväkuntoinen, rakennusoikeutta jäljellä") >= 0.9);
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

    #[test]
    fn property_family_classifies_across_the_nordics() {
        for t in ["omakotitalo", "detached", "Villa", "enebolig", "einbýlishús"] {
            assert_eq!(property_family(t), "house", "{t}");
        }
        for t in ["mökki", "fritidshus", "Hytte", "sumarhús", "holiday house"] {
            assert_eq!(property_family(t), "leisure", "{t}");
        }
        assert_eq!(property_family("farm"), "farm");
        assert_eq!(property_family("garage"), "other");
    }

    #[test]
    fn finnish_token_spec_matches_other_countries_structurally() {
        let spec = Spec {
            property_types: vec!["omakotitalo".into(), "mökki".into()],
            ..Default::default()
        };
        let dk_house = Listing { property_type: Some("detached".into()), price_eur: Some(90_000), ..Default::default() };
        let se_cabin = Listing { property_type: Some("fritidshus".into()), price_eur: Some(90_000), ..Default::default() };
        let is_farm = Listing { property_type: Some("farm".into()), price_eur: Some(90_000), ..Default::default() };
        assert!(passes_structural(&spec, &dk_house), "DK detached should match an omakotitalo/mökki spec");
        assert!(passes_structural(&spec, &se_cabin), "SE fritidshus should match mökki");
        assert!(!passes_structural(&spec, &is_farm), "a farm is neither omakotitalo nor mökki");
    }

    #[test]
    fn own_shore_recognized_across_countries() {
        assert!(own_lake_shore(&Listing { shore: Some("own_shore".into()), ..Default::default() }));
        assert!(own_lake_shore(&Listing { shore: Some("oma_ranta".into()), ..Default::default() }));
        assert!(!own_lake_shore(&Listing { shore: Some("no_shore".into()), ..Default::default() }));
        assert!(!own_lake_shore(&Listing { shore: None, ..Default::default() }));
    }
}
