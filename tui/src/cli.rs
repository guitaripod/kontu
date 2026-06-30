//! Agent-native command line. Every subcommand supports `--json` for structured
//! output, and `--help` documents the surface so an LLM can discover and drive
//! it. With no subcommand the binary prints help.

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use serde_json::json;

use crate::api::KontuClient;
use crate::config::Config;
use crate::cost::{CostDefaults, CostState, HeatingType, Projection, RepaymentType};
use crate::format::{area_opt, int_opt, money, money_opt, num_opt, ppm2_opt, str_opt};
use crate::models::{FilterState, Listing, ListingDetail, ListingsPage, SortColumn};
use crate::risk::{self, RiskAssessment};
use crate::spec::{Pref, Spec};
use crate::{telegram, watch};

/// Resolve cost-model defaults for a listing by its country. Finland keeps the
/// Worker-provided (overridable) defaults; the other Nordic markets use their
/// built-in calibrated seeds. Refuses a country whose model isn't calibrated yet
/// rather than silently costing it with Finnish numbers.
fn defaults_for_listing(worker_fi: &CostDefaults, listing: &Listing) -> Result<CostDefaults> {
    let country = listing.country_enum();
    if !country.cost_calibrated() {
        anyhow::bail!(
            "the cost & risk model for {} ({}) is not calibrated yet",
            country.name(),
            country.code()
        );
    }
    Ok(CostDefaults::resolve(worker_fi, country))
}

#[derive(Parser, Debug)]
#[command(
    name = "kontu",
    version,
    about = "Find and decide on a house to buy in Finland.",
    long_about = "kontu — a Finnish house-hunting tool.\n\nWith NO subcommand it opens an interactive terminal UI. With a subcommand it acts \
as a scriptable, agent-friendly CLI: every command takes --json for machine-readable \
output. Listings, history and photos come from the kontu Cloudflare Worker; the \
total-cost-of-ownership and buyer-risk models run locally.",
    after_help = "EXAMPLES:\n  \
kontu list --municipality Outokumpu --price-max 120000 --shore oma_ranta --json\n  \
kontu show 8002 --json\n  \
kontu cost 8002 --ltv 0.7 --euribor 0.03 --horizon 25 --json\n  \
kontu risk 8002 --json\n  \
kontu compare 8002 8007 8010 --json\n  \
kontu score 8002 80 --deal-breaker\n  \
kontu note 8002 \"Lakeside; book a kuntotutkimus.\"\n  \
kontu pull Outokumpu          (ingest real listings from YOUR IP — the Worker's is blocked)\n\n\
Connection: reads ~/.config/kontu/config.toml, overridable with --server/--token \
or KONTU_SERVER_URL/KONTU_API_TOKEN."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Emit machine-readable JSON instead of human text (use this from agents/scripts)
    #[arg(long, global = true)]
    pub json: bool,

    /// Override the Worker base URL (else config / KONTU_SERVER_URL)
    #[arg(long, global = true)]
    pub server: Option<String>,

    /// Override the API token (else config / KONTU_API_TOKEN)
    #[arg(long, global = true)]
    pub token: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// List listings filtered by exact parameters
    List(ListArgs),
    /// Show full detail for one listing (params, risk, cost, history, notes)
    Show {
        /// Listing id
        id: i64,
    },
    /// Model the total cost of ownership for a listing, with optional overrides
    Cost(CostArgs),
    /// Buyer-risk assessment (0–100 score + deferred-capex flags)
    Risk {
        /// Listing id
        id: i64,
    },
    /// Compare several listings side by side (price, €/m², modelled cost, risk)
    Compare {
        /// Two or more listing ids
        #[arg(required = true, num_args = 1..)]
        ids: Vec<i64>,
    },
    /// Set your personal score (0–100) for a listing
    Score {
        id: i64,
        /// Score 0–100
        score: i32,
        /// Flag this listing as a deal-breaker
        #[arg(long)]
        deal_breaker: bool,
    },
    /// Set a free-text note on a listing
    Note { id: i64, text: String },
    /// Trigger a sync crawl and report crawl state
    Sync,
    /// Print the seeded 2026 cost-model defaults
    Defaults,
    /// Area price statistics for a municipality (price-fairness backbone)
    Market { municipality: String },
    /// Open a listing on its source site in the browser
    Open { id: i64 },
    /// Connectivity + contract self-check against the Worker
    Doctor,
    /// Print the agent playbook (how an LLM should drive this CLI)
    Guide,
    /// Pull real Oikotie listings for a municipality (fetched from YOUR IP) into the Worker
    Pull(PullArgs),
    /// Show or edit your saved house-hunting spec (the criteria `match` ranks against)
    Spec {
        #[command(subcommand)]
        action: Option<SpecAction>,
    },
    /// Find and rank listings by fit to your saved spec (best first)
    Match(MatchArgs),
    /// Render a shareable PNG ownership one-pager for a listing
    Card(CardArgs),
    /// New-listing alerts: poll your spec and push fresh matches to Telegram
    Watch {
        #[command(subcommand)]
        action: Option<WatchAction>,
    },
}

#[derive(Subcommand, Debug)]
pub enum WatchAction {
    /// Run one detection cycle now (pull + match + diff + notify) — the timer's job
    Run(WatchRunArgs),
    /// Set Telegram credentials (bot token from @BotFather; chat id auto-detected)
    Config(WatchConfigArgs),
    /// Send a test message to confirm Telegram delivery works
    Test,
    /// Install a systemd-user timer that runs `kontu watch run` on a schedule
    Install(WatchInstallArgs),
    /// Show watch status (credentials, baseline size, how to enable the timer)
    Status,
}

#[derive(Args, Debug)]
pub struct WatchRunArgs {
    /// Skip the fresh pull and rank already-ingested listings
    #[arg(long)]
    no_pull: bool,
    /// Optional extra fit floor for this run (0–100); the quality gate already
    /// decides which homes are relevant, so this only narrows alerts further
    #[arg(long)]
    min_fit: Option<f64>,
    /// Listings scanned per area before ranking
    #[arg(long, default_value_t = 800)]
    scan: usize,
    /// Mark all current matches as seen WITHOUT alerting (establish a baseline)
    #[arg(long)]
    seed: bool,
}

#[derive(Args, Debug)]
pub struct WatchConfigArgs {
    /// Telegram bot token from @BotFather
    #[arg(long)]
    token: Option<String>,
    /// Telegram chat id (omit to auto-detect from a message you sent the bot)
    #[arg(long)]
    chat_id: Option<String>,
}

#[derive(Args, Debug)]
pub struct WatchInstallArgs {
    /// systemd OnCalendar expression (default: 2-hourly, 08:00–22:00)
    #[arg(long)]
    schedule: Option<String>,
}

#[derive(Args, Debug)]
pub struct CardArgs {
    /// Listing id
    id: i64,
    /// Card language: fi | en
    #[arg(long, default_value = "fi")]
    lang: String,
    /// Output PNG path (default: ~/kontu-<municipality>-<id>.png)
    #[arg(long)]
    out: Option<std::path::PathBuf>,
}

#[derive(Args, Debug)]
pub struct MatchArgs {
    /// Pull fresh listings for your spec from your IP before matching
    #[arg(long)]
    pull: bool,
    /// How many top matches to show
    #[arg(long, default_value_t = 15)]
    limit: usize,
    /// Cap on listings scanned/scored
    #[arg(long, default_value_t = 800)]
    scan: usize,
}

#[derive(Subcommand, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum SpecAction {
    /// Update spec fields (only the flags you pass change)
    Set(SpecSetArgs),
    /// Reset the spec to empty
    Clear,
}

#[derive(Args, Debug)]
pub struct SpecSetArgs {
    #[arg(long)]
    price_max: Option<i64>,
    #[arg(long)]
    price_min: Option<i64>,
    /// Country to search: FI|SE|NO|DK|IS (repeatable); none = all Nordic
    #[arg(long = "country")]
    country: Vec<String>,
    /// Search all Nordic countries (clears the saved country filter)
    #[arg(long = "all-countries")]
    all_countries: bool,
    /// Municipality to search (repeatable); none = anywhere in the chosen countries
    #[arg(long = "area")]
    area: Vec<String>,
    /// Search anywhere (clears saved areas)
    #[arg(long)]
    anywhere: bool,
    /// Property type (repeatable), e.g. omakotitalo, mökki
    #[arg(long = "type")]
    property_type: Vec<String>,
    /// Lakehouse: any|plus|required|avoid
    #[arg(long)]
    shore: Option<String>,
    #[arg(long)]
    min_plot_m2: Option<f64>,
    #[arg(long)]
    min_m2: Option<f64>,
    #[arg(long)]
    min_rooms: Option<f64>,
    #[arg(long)]
    year_min: Option<i32>,
    /// Prefer an owned plot (avoid vuokratontti)
    #[arg(long = "owned-plot", overrides_with = "no_owned_plot")]
    owned_plot: bool,
    /// Stop preferring an owned plot
    #[arg(long = "no-owned-plot")]
    no_owned_plot: bool,
    /// Require working everyday infrastructure (water/sewer/electricity/road)
    #[arg(long = "require-infra", overrides_with = "no_require_infra")]
    require_infra: bool,
    /// Drop the infrastructure requirement
    #[arg(long = "no-require-infra")]
    no_require_infra: bool,
    /// EV charging: any|plus|required|avoid
    #[arg(long)]
    ev: Option<String>,
    /// Fibre internet: any|plus|required|avoid
    #[arg(long)]
    fiber: Option<String>,
    /// Not direct neighbours: any|plus|required|avoid
    #[arg(long)]
    privacy: Option<String>,
    /// Year-round liveable, not a summer mökki: any|plus|required|avoid
    #[arg(long)]
    winterized: Option<String>,
    /// Good condition / move-in (not a renovation project): any|plus|required|avoid
    #[arg(long)]
    condition: Option<String>,
    /// Single-storey only (drop clearly multi-floor houses)
    #[arg(long = "single-floor", overrides_with = "no_single_floor")]
    single_floor: bool,
    /// Allow multi-floor houses
    #[arg(long = "no-single-floor")]
    no_single_floor: bool,
    /// Buy outright with no mortgage (cost model uses LTV 0, no loan interest)
    #[arg(long = "cash", overrides_with = "no_cash")]
    cash: bool,
    /// Model a mortgage instead of a cash purchase
    #[arg(long = "no-cash")]
    no_cash: bool,
    /// Rank toward the lowest total cost of ownership
    #[arg(long = "minimize-tco", overrides_with = "no_minimize_tco")]
    minimize_tco: bool,
    /// Stop ranking toward lowest TCO
    #[arg(long = "no-minimize-tco")]
    no_minimize_tco: bool,
    #[arg(long = "max-dom")]
    max_dom: Option<i64>,
    /// Drop houses with a buyer-risk score above this (0–100) — "sound condition"
    #[arg(long = "max-risk")]
    max_risk: Option<u32>,
    /// Risk ceiling for the near-miss band: sound on-spec homes scoring between
    /// max-risk and this are surfaced + alerted (marked), not dropped; 0 disables
    #[arg(long = "near-miss-risk")]
    near_miss_risk: Option<u32>,
    /// Pin a listing into your options regardless of the gate (repeatable)
    #[arg(long = "pin")]
    pin: Vec<i64>,
    /// Remove a pinned listing (repeatable)
    #[arg(long = "unpin")]
    unpin: Vec<i64>,
    /// Cost-model horizon in years
    #[arg(long)]
    horizon: Option<u32>,
    /// Optional extra fit floor on alerts (0–100; 0 = the quality gate alone decides)
    #[arg(long = "alert-min-fit")]
    alert_min_fit: Option<f64>,
    /// Exclude listings matching this keyword (repeatable)
    #[arg(long = "exclude")]
    exclude: Vec<String>,
    /// Free-text note capturing intent the fields can't
    #[arg(long)]
    note: Option<String>,
}

impl SpecSetArgs {
    fn apply(&self, s: &mut Spec) {
        if let Some(v) = self.price_max {
            s.price_max = Some(v);
        }
        if let Some(v) = self.price_min {
            s.price_min = Some(v);
        }
        if self.all_countries {
            s.countries.clear();
        } else if !self.country.is_empty() {
            s.countries = self.country.iter().map(|c| c.to_uppercase()).collect();
        }
        if self.anywhere {
            s.municipalities.clear();
        }
        if !self.area.is_empty() {
            s.municipalities = self.area.clone();
        }
        if !self.property_type.is_empty() {
            s.property_types = self.property_type.clone();
        }
        if let Some(p) = &self.shore {
            s.shore = Pref::parse(p);
        }
        if self.min_plot_m2.is_some() {
            s.min_plot_m2 = self.min_plot_m2;
        }
        if self.min_m2.is_some() {
            s.min_m2 = self.min_m2;
        }
        if self.min_rooms.is_some() {
            s.min_rooms = self.min_rooms;
        }
        if self.year_min.is_some() {
            s.year_min = self.year_min;
        }
        if self.owned_plot {
            s.owned_plot = true;
        } else if self.no_owned_plot {
            s.owned_plot = false;
        }
        if self.require_infra {
            s.require_infra = true;
        } else if self.no_require_infra {
            s.require_infra = false;
        }
        if let Some(p) = &self.ev {
            s.ev_charging = Pref::parse(p);
        }
        if let Some(p) = &self.fiber {
            s.fiber = Pref::parse(p);
        }
        if let Some(p) = &self.privacy {
            s.privacy = Pref::parse(p);
        }
        if let Some(p) = &self.winterized {
            s.winterized = Pref::parse(p);
        }
        if let Some(p) = &self.condition {
            s.condition = Pref::parse(p);
        }
        if self.single_floor {
            s.single_floor = true;
        } else if self.no_single_floor {
            s.single_floor = false;
        }
        if self.cash {
            s.cash = true;
        } else if self.no_cash {
            s.cash = false;
        }
        if self.minimize_tco {
            s.minimize_tco = true;
        } else if self.no_minimize_tco {
            s.minimize_tco = false;
        }
        if self.max_dom.is_some() {
            s.max_dom = self.max_dom;
        }
        if self.max_risk.is_some() {
            s.max_risk = self.max_risk;
        }
        if let Some(v) = self.near_miss_risk {
            s.near_miss_risk = (v > 0).then_some(v);
        }
        for id in &self.pin {
            if !s.pinned.contains(id) {
                s.pinned.push(*id);
            }
        }
        if !self.unpin.is_empty() {
            s.pinned.retain(|id| !self.unpin.contains(id));
        }
        if let Some(v) = self.horizon {
            s.horizon_years = v;
        }
        if let Some(v) = self.alert_min_fit {
            s.alert_min_fit = v;
        }
        if !self.exclude.is_empty() {
            s.exclude = self.exclude.clone();
        }
        if let Some(n) = &self.note {
            s.notes = n.clone();
        }
    }
}

#[derive(Args, Debug)]
pub struct PullArgs {
    /// Municipality, e.g. Outokumpu. Omit to pull from ALL of Finland (use filters!)
    municipality: Option<String>,
    /// Property type to include (repeatable): omakotitalo, mökki, rivitalo, paritalo, kerrostalo, erillistalo
    #[arg(long = "type")]
    property_types: Vec<String>,
    /// Only lakehouses (own shore or shore right)
    #[arg(long)]
    shore: bool,
    /// Only listings at or below this price (€)
    #[arg(long)]
    price_max: Option<i64>,
    /// Maximum number of listings to pull (per portal)
    #[arg(long, default_value_t = 200)]
    limit: usize,
    /// Which portal(s): oikotie | etuovi | both
    #[arg(long, default_value = "both")]
    portal: String,
    /// Skip per-listing detail enrichment (faster, but no shore-type/condition/heating)
    #[arg(long)]
    shallow: bool,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Country: FI | SE | NO | DK | IS (default: all supported)
    #[arg(long)]
    country: Option<String>,
    #[arg(long)]
    municipality: Option<String>,
    /// Property type: omakotitalo | paritalo | rivitalo | kerrostalo | mökki
    #[arg(long = "type")]
    property_type: Option<String>,
    /// Holding form: kiinteisto | asunto_osake
    #[arg(long)]
    holding: Option<String>,
    #[arg(long)]
    price_min: Option<i64>,
    #[arg(long)]
    price_max: Option<i64>,
    #[arg(long)]
    m2_min: Option<f64>,
    #[arg(long)]
    m2_max: Option<f64>,
    #[arg(long)]
    rooms_min: Option<f64>,
    #[arg(long)]
    year_min: Option<i32>,
    /// Shore: oma_ranta | rantaoikeus | ei_rantaa
    #[arg(long)]
    shore: Option<String>,
    #[arg(long)]
    heating: Option<String>,
    /// Max energy class to allow (A best … G worst), e.g. C
    #[arg(long = "energy-class-max")]
    energy_class_max: Option<String>,
    /// Plot ownership: oma | vuokra
    #[arg(long)]
    plot: Option<String>,
    #[arg(long = "max-dom")]
    max_dom: Option<i64>,
    /// Drop listings whose text matches a keyword (repeatable)
    #[arg(long)]
    exclude: Vec<String>,
    /// Only listings whose price has dropped since first seen
    #[arg(long)]
    price_dropped: bool,
    /// Free-text search
    #[arg(long)]
    text: Option<String>,
    /// Sort: price | ppm2 | size | year | dom | risk | score
    #[arg(long, default_value = "price")]
    sort: String,
    /// Sort descending
    #[arg(long)]
    desc: bool,
    #[arg(long, default_value_t = 50)]
    limit: u32,
}

impl ListArgs {
    fn into_query(self) -> (FilterState, SortColumn, bool, u32) {
        let filter = FilterState {
            country: self.country.map(|c| c.to_uppercase()),
            municipality: self.municipality,
            property_type: self.property_type,
            holding_form: self.holding,
            price_min: self.price_min,
            price_max: self.price_max,
            m2_min: self.m2_min,
            m2_max: self.m2_max,
            rooms_min: self.rooms_min,
            year_min: self.year_min,
            shore: self.shore,
            heating_type: self.heating,
            energy_class_max: self.energy_class_max,
            plot_ownership: self.plot,
            max_days_on_market: self.max_dom,
            exclude_keywords: self.exclude,
            price_dropped: self.price_dropped,
            text: self.text,
        };
        (filter, parse_sort(&self.sort), self.desc, self.limit)
    }
}

#[derive(Args, Debug)]
pub struct CostArgs {
    /// Listing id
    id: i64,
    /// Override purchase price (€)
    #[arg(long)]
    price: Option<f64>,
    /// Loan-to-value, 0..0.95
    #[arg(long)]
    ltv: Option<f64>,
    /// 12-month Euribor, e.g. 0.029
    #[arg(long)]
    euribor: Option<f64>,
    /// Bank margin, e.g. 0.005
    #[arg(long)]
    margin: Option<f64>,
    /// Loan term in years
    #[arg(long)]
    term: Option<u32>,
    /// Holding horizon in years
    #[arg(long)]
    horizon: Option<u32>,
    /// Real discount rate (opportunity cost), e.g. 0.03
    #[arg(long)]
    discount: Option<f64>,
    /// Heating: kaukolampo | maalampo | oljy | sahko | puu | ivlp
    #[arg(long)]
    heating: Option<String>,
    /// Repayment: annuiteetti | tasalyhennys | kiintea
    #[arg(long)]
    repayment: Option<String>,
    /// Monthly housing-company charge (hoito + rahoitusvastike), €/mo
    #[arg(long)]
    vastike: Option<f64>,
    /// Include the year-by-year schedule in the output
    #[arg(long)]
    schedule: bool,
}

impl CostArgs {
    fn apply(&self, cs: &mut CostState) {
        if let Some(v) = self.price {
            cs.price = v;
            cs.debt_free_price = v;
        }
        if let Some(v) = self.ltv {
            cs.ltv = v;
        }
        if let Some(v) = self.euribor {
            cs.euribor = v;
        }
        if let Some(v) = self.margin {
            cs.margin = v;
        }
        if let Some(v) = self.term {
            cs.term_years = v;
        }
        if let Some(v) = self.horizon {
            cs.horizon = v;
        }
        if let Some(v) = self.discount {
            cs.real_discount = v;
        }
        if let Some(h) = &self.heating {
            cs.heating = parse_heating(h);
        }
        if let Some(r) = &self.repayment {
            cs.repayment = parse_repayment(r);
        }
        if let Some(v) = self.vastike {
            cs.vastike = v;
        }
    }
}

pub async fn run(command: Command, client: &KontuClient, json: bool) -> Result<()> {
    match command {
        Command::List(a) => {
            let (filter, sort, desc, limit) = a.into_query();
            let page = if matches!(sort, SortColumn::RiskScore) {
                risk_sorted_page(client, &filter, desc, limit).await?
            } else {
                client.list_listings(&filter, sort, desc, limit, 0).await?
            };
            if json {
                emit(&page)?;
            } else {
                print_list(&page.listings, page.total);
            }
        }
        Command::Show { id } => {
            let detail = client.get_listing(id).await?;
            let worker_defaults = client.cost_defaults().await.unwrap_or_default();
            let defaults = defaults_for_listing(&worker_defaults, &detail.listing)?;
            let assessment = assess(&detail);
            let mut cs = CostState::from_defaults(&defaults);
            cs.apply_listing(&detail.listing, &assessment, &defaults);
            apply_spec_financing(&mut cs);
            let proj = cs.project(&defaults);
            if json {
                emit(&json!({
                    "listing": detail.listing,
                    "events": detail.events,
                    "photos": detail.photos.len(),
                    "dossier": detail.dossier,
                    "note": detail.note,
                    "score": detail.score,
                    "tags": detail.tags,
                    "risk": assessment,
                    "cost": cost_summary(&proj),
                }))?;
            } else {
                print_show(&detail, &assessment, &proj);
            }
        }
        Command::Cost(a) => {
            let detail = client.get_listing(a.id).await?;
            let worker_defaults = client.cost_defaults().await.unwrap_or_default();
            let defaults = defaults_for_listing(&worker_defaults, &detail.listing)?;
            let assessment = assess(&detail);
            let mut cs = CostState::from_defaults(&defaults);
            cs.apply_listing(&detail.listing, &assessment, &defaults);
            apply_spec_financing(&mut cs);
            a.apply(&mut cs);
            let proj = cs.project(&defaults);
            if json {
                if a.schedule {
                    emit(&proj)?;
                } else {
                    emit(&cost_summary(&proj))?;
                }
            } else {
                print_cost(&detail.listing, &proj, a.schedule);
            }
        }
        Command::Risk { id } => {
            let detail = client.get_listing(id).await?;
            let assessment = assess(&detail);
            if json {
                emit(&assessment)?;
            } else {
                print_risk(&detail.listing, &assessment);
            }
        }
        Command::Compare { ids } => {
            let worker_defaults = client.cost_defaults().await.unwrap_or_default();
            let mut rows = Vec::new();
            for id in ids {
                let detail = client.get_listing(id).await?;
                let defaults = defaults_for_listing(&worker_defaults, &detail.listing)?;
                let assessment = assess(&detail);
                let mut cs = CostState::from_defaults(&defaults);
                cs.apply_listing(&detail.listing, &assessment, &defaults);
                apply_spec_financing(&mut cs);
                let proj = cs.project(&defaults);
                rows.push((detail.listing, assessment.score, proj.npv_cost, proj.equivalent_monthly));
            }
            if json {
                emit(&json!(rows
                    .iter()
                    .map(|(l, score, npv, mo)| json!({
                        "id": l.id, "title": l.title(), "price_eur": l.price_eur,
                        "ppm2": l.effective_ppm2(), "risk": score, "npv_cost": npv, "monthly": mo,
                    }))
                    .collect::<Vec<_>>()))?;
            } else {
                print_compare(&rows);
            }
        }
        Command::Score { id, score, deal_breaker } => {
            client.set_score(id, score, deal_breaker).await?;
            ok(json, format!("score {score} set on #{id}"));
        }
        Command::Note { id, text } => {
            client.set_note(id, &text).await?;
            ok(json, format!("note set on #{id}"));
        }
        Command::Sync => {
            let v = client.trigger_sync().await?;
            emit_or(&v, json, "sync triggered");
        }
        Command::Defaults => {
            let d = client.cost_defaults().await?;
            if json {
                emit(&d)?;
            } else {
                print_defaults(&d);
            }
        }
        Command::Market { municipality } => {
            let v = client.market(&municipality).await?;
            emit(&v)?;
        }
        Command::Open { id } => {
            let detail = client.get_listing(id).await?;
            open::that_detached(&detail.listing.url)?;
            ok(json, format!("opened #{id} ({})", detail.listing.url));
        }
        Command::Doctor => {
            let healthy = client.health().await?;
            let defaults = client.cost_defaults().await.map(|_| true).unwrap_or(false);
            let listings = client
                .list_listings(&FilterState::default(), SortColumn::Price, false, 1, 0)
                .await
                .map(|p| p.total)
                .unwrap_or(-1);
            if json {
                emit(&json!({ "health": healthy, "cost_defaults": defaults, "listings_total": listings }))?;
            } else {
                println!(
                    "health={} cost_defaults={} listings={}",
                    healthy, defaults, listings
                );
            }
        }
        Command::Guide => {
            print!("{}", include_str!("../../AGENTS.md"));
        }
        Command::Pull(a) => {
            let scope = a.municipality.clone().unwrap_or_else(|| "all of Finland".into());
            let r = pull_portals(
                client,
                &a.portal,
                a.municipality.as_deref(),
                &a.property_types,
                a.shore,
                a.price_max,
                a.limit,
                !a.shallow,
                &scope,
            )
            .await?;
            if json {
                emit(&r)?;
            } else {
                print_import(&r);
            }
        }
        Command::Spec { action } => match action {
            None => {
                let s = Spec::load()?;
                if json {
                    emit(&s)?;
                } else {
                    print_spec(&s);
                }
            }
            Some(SpecAction::Clear) => {
                Spec::default().save()?;
                ok(json, "spec cleared".into());
            }
            Some(SpecAction::Set(a)) => {
                let mut s = Spec::load()?;
                a.apply(&mut s);
                s.save()?;
                if json {
                    emit(&s)?;
                } else {
                    print_spec(&s);
                }
            }
        },
        Command::Match(a) => {
            let spec = Spec::load()?;
            if a.pull {
                pull_spec(client, &spec, a.scan).await;
            }
            let defaults = client.cost_defaults().await.unwrap_or_default();
            let mut listings = fetch_spec_listings(client, &spec, a.scan).await?;
            ensure_pinned(client, &spec, &mut listings).await;
            let ranked = crate::matching::rank(&spec, listings, &defaults);
            let top: Vec<_> = ranked.into_iter().take(a.limit).collect();
            if json {
                emit(&top)?;
            } else {
                print_matches(&top);
            }
        }
        Command::Watch { action } => handle_watch(action, client, json).await?,
        Command::Card(a) => {
            let path =
                crate::card::render_card(client, a.id, crate::card::Lang::parse(&a.lang), a.out)
                    .await?;
            ok(json, format!("wrote {}", path.display()));
        }
    }
    Ok(())
}

/// Pull fresh listings for the spec from this machine's IP — per-municipality when
/// the spec names areas (so each gets a real server-side filter), else nationwide.
async fn pull_spec(client: &KontuClient, spec: &Spec, scan: usize) {
    let shore = matches!(spec.shore, Pref::Required | Pref::Plus);
    if spec.municipalities.is_empty() {
        let _ = pull_portals(
            client, "both", None, &spec.property_types, shore, spec.price_max, scan, true,
            "your spec",
        )
        .await;
    } else {
        for m in &spec.municipalities {
            let _ = pull_portals(
                client, "both", Some(m.as_str()), &spec.property_types, shore, spec.price_max, scan,
                true, m,
            )
            .await;
        }
    }
}

async fn handle_watch(action: Option<WatchAction>, client: &KontuClient, json: bool) -> Result<()> {
    match action.unwrap_or(WatchAction::Status) {
        WatchAction::Config(a) => watch_config(a, json).await,
        WatchAction::Test => watch_test(json).await,
        WatchAction::Install(a) => {
            let cfg = Config::load()?;
            let configured = !cfg.telegram_token.is_empty() && !cfg.telegram_chat_id.is_empty();
            let summary = watch::install_timer(a.schedule, configured)?;
            ok(json, summary);
            Ok(())
        }
        WatchAction::Status => {
            watch_status(json)?;
            Ok(())
        }
        WatchAction::Run(a) => watch_run(a, client, json).await,
    }
}

/// Persist Telegram credentials; auto-detect the chat id from a message the user
/// sent the bot when only the token is given.
async fn watch_config(a: WatchConfigArgs, json: bool) -> Result<()> {
    let mut cfg = Config::load_raw()?;
    if let Some(t) = a.token {
        cfg.telegram_token = t.trim().to_string();
    }
    match a.chat_id {
        Some(c) => cfg.telegram_chat_id = c.trim().to_string(),
        None if cfg.telegram_chat_id.is_empty() && !cfg.telegram_token.is_empty() => {
            match telegram::detect_chat_id(&cfg.telegram_token).await {
                Ok(id) => cfg.telegram_chat_id = id,
                Err(e) => eprintln!("kontu: chat id not detected ({e})"),
            }
        }
        None => {}
    }
    cfg.save()?;
    let configured = !cfg.telegram_token.is_empty() && !cfg.telegram_chat_id.is_empty();
    if json {
        emit(&json!({
            "telegram_token_set": !cfg.telegram_token.is_empty(),
            "telegram_chat_id": cfg.telegram_chat_id,
            "configured": configured,
        }))
    } else {
        println!(
            "telegram token: {}\ntelegram chat id: {}\n{}",
            if cfg.telegram_token.is_empty() { "unset" } else { "set" },
            if cfg.telegram_chat_id.is_empty() { "unset" } else { &cfg.telegram_chat_id },
            if configured {
                "ready — `kontu watch test` to confirm, then `kontu watch install`"
            } else {
                "message your bot once, then run `kontu watch config` to auto-detect the chat id"
            }
        );
        Ok(())
    }
}

async fn watch_test(json: bool) -> Result<()> {
    let cfg = Config::load()?;
    require_telegram(&cfg)?;
    telegram::send_message(
        &cfg.telegram_token,
        &cfg.telegram_chat_id,
        "✅ <b>kontu</b> watch is connected. New matches to your spec will land here.",
    )
    .await?;
    ok(json, "sent a test message to Telegram".into());
    Ok(())
}

fn watch_status(json: bool) -> Result<()> {
    let cfg = Config::load()?;
    let spec = Spec::load()?;
    let seen = watch::load_seen().unwrap_or_default();
    let configured = !cfg.telegram_token.is_empty() && !cfg.telegram_chat_id.is_empty();
    if json {
        emit(&json!({
            "configured": configured,
            "telegram_chat_id": cfg.telegram_chat_id,
            "baseline_seen": seen.len(),
            "spec_is_empty": spec.is_empty(),
        }))
    } else {
        println!(
            "telegram: {}\nbaseline (already-seen listings): {}\nspec: {}\n\nsetup: kontu watch config --token <BotFather token> → message the bot → kontu watch config → kontu watch run --seed → kontu watch install",
            if configured { "configured" } else { "not configured" },
            seen.len(),
            if spec.is_empty() { "empty (run `kontu spec set …`)" } else { "set" },
        );
        Ok(())
    }
}

/// One detection cycle: pull → match → diff against the baseline → alert on new.
async fn watch_run(a: WatchRunArgs, client: &KontuClient, json: bool) -> Result<()> {
    let cfg = Config::load()?;
    require_telegram(&cfg)?;
    let spec = Spec::load()?;
    if spec.is_empty() {
        anyhow::bail!("spec is empty — set criteria with `kontu spec set …` first");
    }
    if !a.no_pull {
        pull_spec(client, &spec, a.scan).await;
    }
    let defaults = client.cost_defaults().await.unwrap_or_default();
    let fit_floor = a.min_fit.unwrap_or(spec.alert_min_fit);
    // A transient backend blip (e.g. Cloudflare 503) must not fail the whole cycle
    // and spam systemd — skip quietly and let the next scheduled run recover.
    let listings = match fetch_spec_listings(client, &spec, a.scan).await {
        Ok(l) => l,
        Err(e) => {
            ok(json, format!("skipped this cycle — backend unavailable (transient): {e}"));
            return Ok(());
        }
    };
    // Drop listings the pull hasn't seen for a while — almost always sold/delisted
    // (the DB never marks status, so staleness is the only signal). The window is
    // country-aware: Finland is re-pulled reliably every cycle from Oikotie/Etuovi, so
    // 3 missed days means gone; the other Nordic portals are bot-gated and only polled
    // once daily from the residential IP, so a transient block (a single empty Booli
    // response) must NOT wipe a whole country's stock — give them a fortnight's grace.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    const FI_WINDOW_SECS: i64 = 3 * 24 * 3600;
    const NORDIC_WINDOW_SECS: i64 = 14 * 24 * 3600;
    let mut live: Vec<_> = listings
        .into_iter()
        .filter(|l| {
            if now == 0 {
                return true;
            }
            let window = if l.country_enum() == crate::country::Country::Fi {
                FI_WINDOW_SECS
            } else {
                NORDIC_WINDOW_SECS
            };
            now - l.last_seen <= window
        })
        .collect();
    // Pinned listings must never be dropped by the fetch or the freshness window —
    // otherwise a pin outside the candidate set would be ranked out AND then pruned.
    ensure_pinned(client, &spec, &mut live).await;
    // Bound the off-spec "value outliers" lane: a shore-required spec leaves most of
    // Finland's cheap inland houses underpriced-but-off-spec, so cap it — otherwise the
    // lane floods the page and the publish cycle. The lane is about the steepest
    // discounts vs the local market, so keep the lowest fairness ratios (biggest
    // steals); unknown ratios sort last. Gate / near-miss / pin are never capped.
    // Headroom above the ~12 the site shows: the Worker (which owns the geodata bug
    // signal) drops the bug-ridden and leads with the least buggy, so it needs spare
    // candidates to choose from — the ranker can't see bug pressure to pre-pick them.
    const OUTLIER_LIMIT: usize = 24;
    let ranked = crate::matching::rank(&spec, live, &defaults);
    let (outliers, matches): (Vec<_>, Vec<_>) = ranked.into_iter().partition(|m| m.value_outlier);
    // The fit floor narrows ALERTS — it gates the gate/near-miss/pin set only. It must NOT
    // prune the value-outlier showcase, which is explicitly the "what's possible" lane and
    // is bounded separately by select_outliers. A pin is never dropped by a fit floor.
    let mut matches: Vec<_> = matches
        .into_iter()
        .filter(|m| m.pinned || m.score >= fit_floor)
        .collect();
    matches.extend(select_outliers(outliers, OUTLIER_LIMIT));

    let mut seen = watch::load_seen()?;
    if a.seed {
        for m in &matches {
            seen.insert(m.id);
        }
        watch::save_seen(&seen)?;
        ok(json, format!("seeded {} current matches as baseline (no alerts sent)", matches.len()));
        return Ok(());
    }

    let mut published = 0usize;
    let mut alerted = 0usize;
    let mut failed = 0usize;
    for m in &matches {
        let tier = if m.pinned {
            "pin"
        } else if m.near_miss {
            "near_miss"
        } else if m.value_outlier {
            "outlier"
        } else {
            "gate"
        };
        let do_alert = tier == "gate" && !seen.contains(&m.id);
        match publish_listing(client, &cfg, m, &defaults, spec.horizon_years, tier, do_alert).await {
            Ok(()) => {
                published += 1;
                if do_alert {
                    alerted += 1;
                    seen.insert(m.id);
                }
            }
            Err(e) => {
                failed += 1;
                eprintln!("kontu: publish failed for {}: {e}", m.id);
            }
        }
    }
    watch::save_seen(&seen)?;

    // Prune pages for listings that left the showcase (sold / no longer passing).
    // Guard on a healthy run with a non-empty result so a bad pull never wipes the site.
    let mut pruned = 0usize;
    if failed == 0 && !matches.is_empty() {
        let live_ids: std::collections::HashSet<i64> = matches.iter().map(|m| m.id).collect();
        match client.published_ids().await {
            Ok(existing) => {
                for id in existing.into_iter().filter(|id| !live_ids.contains(id)) {
                    match client.delete_published(id).await {
                        Ok(()) => pruned += 1,
                        Err(e) => eprintln!("kontu: prune delete failed for {id}: {e}"),
                    }
                }
            }
            Err(e) => eprintln!("kontu: prune skipped (published_ids failed): {e}"),
        }
    }

    if json {
        emit(&json!({ "checked": matches.len(), "published": published, "alerted": alerted, "pruned": pruned, "failed": failed }))
    } else {
        println!(
            "checked {} matches · {published} published · {alerted} alerted · {pruned} pruned{}",
            matches.len(),
            if failed > 0 { format!(" · {failed} failed") } else { String::new() }
        );
        Ok(())
    }
}

fn require_telegram(cfg: &Config) -> Result<()> {
    if cfg.telegram_token.is_empty() || cfg.telegram_chat_id.is_empty() {
        anyhow::bail!(
            "telegram not configured — run `kontu watch config --token <BotFather token>`, message your bot, then `kontu watch config`"
        );
    }
    Ok(())
}

/// Fetch the candidate listings the spec ranks over. Fetched per
/// (country × municipality) and merged, so a cheapest-N truncation can never drop a
/// relevant listing — a flood of cheap plots in one country can't crowd out houses
/// in another. The property TYPE is NOT filtered server-side: the spec types are
/// Finnish tokens (omakotitalo/mökki) that wouldn't match the normalized SE/NO/DK/IS
/// types, so the matcher's `property_family` does the typing on the full candidate set.
async fn fetch_spec_listings(client: &KontuClient, spec: &Spec, scan: usize) -> Result<Vec<Listing>> {
    let base = spec_to_filter(spec);
    let countries: Vec<Option<String>> = if spec.countries.is_empty() {
        crate::country::Country::ALL.iter().map(|c| Some(c.code().to_string())).collect()
    } else {
        spec.countries.iter().map(|c| Some(c.clone())).collect()
    };
    let munis: Vec<Option<String>> = if spec.municipalities.is_empty() {
        vec![None]
    } else {
        spec.municipalities.iter().map(|m| Some(m.clone())).collect()
    };
    let mut all = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for c in &countries {
        for m in &munis {
            let mut f = base.clone();
            f.country = c.clone();
            f.municipality = m.clone();
            f.property_type = None;
            let page = client.list_listings(&f, SortColumn::Price, false, scan as u32, 0).await?;
            for l in page.listings {
                if seen.insert(l.id) {
                    all.push(l);
                }
            }
        }
    }
    Ok(all)
}

/// Pick the value-outlier lane the showcase displays: country-balanced, best-first.
/// Within a country the deepest discount leads (fairness ratio ascending; the
/// non-FI markets without a sold-price benchmark fall back to the lowest asking
/// price). Across countries we round-robin, so the cross-Nordic "what's possible"
/// view shows finds from every market instead of being swept by one country's
/// steals. Caps the total so the lane never floods the page or the publish cycle.
fn select_outliers(outliers: Vec<crate::matching::Scored>, limit: usize) -> Vec<crate::matching::Scored> {
    use std::collections::BTreeMap;
    let mut by_country: BTreeMap<String, Vec<crate::matching::Scored>> = BTreeMap::new();
    for o in outliers {
        by_country.entry(o.country.clone()).or_default().push(o);
    }
    for lane in by_country.values_mut() {
        lane.sort_by(|a, b| {
            let av = a.fairness_ratio.unwrap_or(f64::INFINITY);
            let bv = b.fairness_ratio.unwrap_or(f64::INFINITY);
            av.partial_cmp(&bv)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.price_eur.unwrap_or(i64::MAX).cmp(&b.price_eur.unwrap_or(i64::MAX)))
        });
    }
    let mut lanes: Vec<_> = by_country.into_values().map(|v| v.into_iter()).collect();
    let mut out = Vec::with_capacity(limit);
    while out.len() < limit {
        let mut progressed = false;
        for lane in lanes.iter_mut() {
            if let Some(item) = lane.next() {
                out.push(item);
                progressed = true;
                if out.len() >= limit {
                    break;
                }
            }
        }
        if !progressed {
            break;
        }
    }
    out
}

/// Ensure every pinned listing is in the candidate set even if it falls outside
/// the spec's server-side filters (e.g. priced above the cap), so a pin is never
/// silently dropped before ranking. Best-effort: a fetch failure just skips it.
async fn ensure_pinned(client: &KontuClient, spec: &Spec, listings: &mut Vec<Listing>) {
    for id in &spec.pinned {
        if listings.iter().any(|l| l.id == *id) {
            continue;
        }
        if let Ok(detail) = client.get_listing(*id).await {
            listings.push(detail.listing);
        }
    }
}

fn spec_to_filter(spec: &Spec) -> FilterState {
    FilterState {
        municipality: (spec.municipalities.len() == 1).then(|| spec.municipalities[0].clone()),
        property_type: (spec.property_types.len() == 1).then(|| spec.property_types[0].clone()),
        price_min: spec.price_min,
        price_max: spec.price_max,
        m2_min: spec.min_m2,
        rooms_min: spec.min_rooms,
        year_min: spec.year_min,
        max_days_on_market: spec.max_dom,
        exclude_keywords: spec.exclude.clone(),
        ..Default::default()
    }
}

fn print_matches(top: &[crate::matching::Scored]) {
    if top.is_empty() {
        println!("no matches — try `kontu match --pull`, widen the spec, or check `kontu spec`.");
        return;
    }
    println!(
        "{:>6} {:>4} {:<22} {:<13} {:>9} {:>4} {:>8}  WHY",
        "ID", "FIT", "PLACE", "WHERE", "PRICE", "RSK", "€/MO*"
    );
    for m in top {
        println!(
            "{:>6} {:>4.0} {:<22} {:<13} {:>9} {:>4} {:>8}  {}",
            m.id,
            m.score,
            trunc(&m.title, 22),
            trunc(m.municipality.as_deref().unwrap_or("?"), 13),
            money_opt(m.price_eur),
            m.risk,
            money(m.monthly_living),
            m.reasons.join(", "),
        );
    }
    println!("* €/MO = monthly running cost (cost of living). open one with: kontu open <id>");
}

#[allow(clippy::too_many_arguments)]
async fn pull_portals(
    client: &KontuClient,
    portal: &str,
    muni: Option<&str>,
    types: &[String],
    shore: bool,
    price_max: Option<i64>,
    limit: usize,
    deep: bool,
    scope: &str,
) -> anyhow::Result<serde_json::Value> {
    let keys = ["received", "inserted", "updated", "skipped"];
    let mut t = [0u64; 4];
    let accumulate = |r: &serde_json::Value, t: &mut [u64; 4]| {
        for (i, k) in keys.iter().enumerate() {
            t[i] += r.get(*k).and_then(serde_json::Value::as_u64).unwrap_or(0);
        }
    };
    if portal != "etuovi" {
        eprintln!("pulling {scope} from oikotie…");
        match crate::ingest::pull_oikotie(client, muni, types, shore, price_max, limit, deep).await {
            Ok(r) => accumulate(&r, &mut t),
            Err(e) => eprintln!("  oikotie: {e}"),
        }
    }
    if portal != "oikotie" {
        eprintln!("pulling {scope} from etuovi…");
        match crate::ingest::pull_etuovi(client, muni, types, shore, price_max, limit).await {
            Ok(r) => accumulate(&r, &mut t),
            Err(e) => eprintln!("  etuovi: {e}"),
        }
    }
    Ok(serde_json::json!({ "received": t[0], "inserted": t[1], "updated": t[2], "skipped": t[3] }))
}

fn print_import(r: &serde_json::Value) {
    let n = |k: &str| r.get(k).and_then(serde_json::Value::as_u64).unwrap_or(0);
    println!(
        "imported {} listings: {} new, {} updated, {} skipped",
        n("received"),
        n("inserted"),
        n("updated"),
        n("skipped")
    );
}

fn opt_i<T: ToString>(v: Option<T>) -> String {
    v.map(|x| x.to_string()).unwrap_or_else(|| "—".into())
}

fn print_spec(s: &Spec) {
    let areas = if s.municipalities.is_empty() {
        "anywhere in Finland".to_string()
    } else {
        s.municipalities.join(", ")
    };
    let types = if s.property_types.is_empty() {
        "any".to_string()
    } else {
        s.property_types.join(", ")
    };
    println!("areas      {areas}");
    println!("price      {} – {}", money_opt(s.price_min), money_opt(s.price_max));
    println!("type       {types}");
    println!("shore      {:?}   winterized {:?}   condition {:?}", s.shore, s.winterized, s.condition);
    println!("privacy    {:?}   ev {:?}   fiber {:?}", s.privacy, s.ev_charging, s.fiber);
    let mut flags = Vec::new();
    if s.owned_plot {
        flags.push("owned-plot");
    }
    if s.require_infra {
        flags.push("infra-required");
    }
    if s.single_floor {
        flags.push("single-floor");
    }
    if s.minimize_tco {
        flags.push("minimize-TCO");
    }
    if s.cash {
        flags.push("cash-purchase");
    }
    let alert = if s.alert_min_fit > 0.0 {
        format!("alert: gate + fit ≥ {:.0}", s.alert_min_fit)
    } else {
        "alert: quality gate".to_string()
    };
    flags.push(&alert);
    if !flags.is_empty() {
        println!("flags      {}", flags.join(", "));
    }
    let risk_str = match (s.max_risk, s.near_miss_risk) {
        (Some(m), Some(n)) => format!("{m}  (near-miss ≤ {n})"),
        (Some(m), None) => m.to_string(),
        (None, Some(n)) => format!("—  (near-miss ≤ {n})"),
        (None, None) => "—".to_string(),
    };
    println!(
        "limits     plot ≥ {} m² · area ≥ {} m² · year ≥ {} · rooms ≥ {} · dom ≤ {} · risk ≤ {}",
        opt_i(s.min_plot_m2.map(|v| v as i64)),
        opt_i(s.min_m2.map(|v| v as i64)),
        opt_i(s.year_min),
        opt_i(s.min_rooms),
        opt_i(s.max_dom),
        risk_str,
    );
    if !s.pinned.is_empty() {
        println!(
            "pinned     {}",
            s.pinned.iter().map(|id| format!("#{id}")).collect::<Vec<_>>().join(", ")
        );
    }
    println!("horizon    {} yr", s.horizon_years);
    if !s.exclude.is_empty() {
        println!("exclude    {}", s.exclude.join(", "));
    }
    if !s.notes.is_empty() {
        println!("notes      {}", s.notes);
    }
    if s.is_empty() {
        println!("(empty — set it with `kontu spec set ...`)");
    }
}

/// `list --sort risk` over the REAL computed RiskScore. The Worker's SQL proxy
/// (description token count) is misleading, so pull a generous candidate window,
/// assess each listing locally, then sort and truncate. The window is capped at
/// `RISK_CANDIDATES` cheapest matches — ample for any single municipality.
async fn risk_sorted_page(
    client: &KontuClient,
    filter: &FilterState,
    desc: bool,
    limit: u32,
) -> anyhow::Result<ListingsPage> {
    const RISK_CANDIDATES: u32 = 1000;
    let mut page = client
        .list_listings(filter, SortColumn::Price, false, RISK_CANDIDATES, 0)
        .await?;
    page.listings.sort_by_key(|l| {
        // "ei_rantaa" contains "ranta", so match the actual waterfront kinds.
        let near_water = l
            .shore
            .as_deref()
            .map(|s| s.contains("oma_ranta") || s.contains("rantaoik"))
            .unwrap_or(false);
        let score = risk::assess(&l.to_risk_input(near_water), 2026).score;
        if desc { u32::MAX - score } else { score }
    });
    page.listings.truncate(limit as usize);
    Ok(page)
}

/// Apply the saved spec's financing stance (cash purchase → LTV 0, no loan
/// interest) as the cost-model default, so drill-downs match how `match` ranked.
fn apply_spec_financing(cs: &mut CostState) {
    if Spec::load().map(|s| s.cash).unwrap_or(false) {
        cs.ltv = 0.0;
    }
}

/// Publish a newly-validated listing's public web page, then push a Telegram photo
/// card whose button deep-links into the web app at that listing.
async fn publish_listing(
    client: &KontuClient,
    cfg: &Config,
    m: &crate::matching::Scored,
    defaults: &crate::cost::CostDefaults,
    horizon: u32,
    tier: &str,
    alert: bool,
) -> Result<()> {
    let detail = client.get_listing(m.id).await?;
    let assessment = assess(&detail);
    let mut cs = CostState::from_defaults(defaults);
    cs.apply_listing(&detail.listing, &assessment, defaults);
    apply_spec_financing(&mut cs);
    cs.horizon = horizon;
    let proj = cs.project(defaults);
    // Finland's HD gallery is scraped from the Oikotie detail page at publish time. The
    // other Nordic portals don't expose that, but their ingesters record a cover URL, so
    // fall back to the stored photos served through the Worker's lazy R2 route — otherwise
    // every cross-Nordic card publishes with a blank image.
    let mut gallery = crate::ingest::fetch_gallery(&detail.listing.url).await;
    if gallery.is_empty() {
        let base = cfg.webapp_url.trim_end_matches('/');
        gallery = detail
            .photos
            .iter()
            .map(|p| format!("{base}/api/photos/{}", p.r2_key))
            .collect();
    }
    let payload = build_publish_payload(&detail, &assessment, &proj, &gallery, horizon, tier, &m.off_spec);
    client.publish_page(m.id, tier, payload).await?;

    if !alert {
        return Ok(());
    }
    let link = format!("{}/kontu/{}", cfg.webapp_url.trim_end_matches('/'), m.id);
    let caption = alert_caption(&detail.listing, m);
    match gallery.first() {
        Some(photo) => {
            telegram::send_photo_with_button(
                &cfg.telegram_token, &cfg.telegram_chat_id, photo, &caption, "Avaa kohde →", &link,
            )
            .await
        }
        None => {
            telegram::send_message_with_button(
                &cfg.telegram_token, &cfg.telegram_chat_id, &caption, "Avaa kohde →", &link,
            )
            .await
        }
    }
}

fn alert_caption(l: &Listing, m: &crate::matching::Scored) -> String {
    let mut bits = Vec::new();
    if let Some(a) = l.living_area_m2 {
        bits.push(format!("{a:.0} m²"));
    }
    if let Some(y) = l.year_built {
        bits.push(y.to_string());
    }
    if let Some(c) = &l.condition_class {
        bits.push(format!("kunto {c}"));
    }
    format!(
        "🏡 <b>{}</b>\n📍 {}\n💶 {} · ~{} €/kk · riski {}\n{}\n\n✅ Uusi validoitu kohde",
        telegram::escape(&l.title()),
        telegram::escape(l.municipality.as_deref().unwrap_or("?")),
        telegram::escape(&money_opt(l.price_eur)),
        m.monthly_living.round() as i64,
        m.risk,
        telegram::escape(&bits.join(" · ")),
    )
}

fn publish_reasons(l: &Listing, tier: &str, off_spec: &[String]) -> Vec<String> {
    let mut r = Vec::new();
    if let Some(c) = &l.condition_class {
        r.push(format!("Kuntoluokka {c}"));
    }
    if l.shore.as_deref().map(|s| s.contains("oma_ranta")).unwrap_or(false) {
        r.push("Oma järvenranta".to_string());
    }
    if let Some(y) = l.year_built {
        r.push(format!("Rakennettu {y}"));
    }
    r.push("Käteiskauppa — ei lainaa, ei vastiketta, ei tonttivuokraa".to_string());
    match tier {
        "gate" => r.push("Läpäisi kontun laatuseulan".to_string()),
        "outlier" => {
            r.push("Selvästi alle alueen mediaanihinnan — poikkeuksellinen hinta".to_string());
            if !off_spec.is_empty() {
                r.push(format!("Ei täytä kaikkia toiveita: {}", off_spec.join(", ")));
            }
        }
        _ => r.push("Täyttää kaikki pakolliset kriteerit — vain ostajan riski yli seulan rajan".to_string()),
    }
    r
}

fn build_publish_payload(
    detail: &ListingDetail,
    assessment: &RiskAssessment,
    proj: &Projection,
    gallery: &[String],
    horizon: u32,
    tier: &str,
    off_spec: &[String],
) -> serde_json::Value {
    let l = &detail.listing;
    let lat = l
        .lat
        .or_else(|| detail.dossier.as_ref().and_then(|d| d.get("lat")).and_then(serde_json::Value::as_f64));
    let lon = l
        .lon
        .or_else(|| detail.dossier.as_ref().and_then(|d| d.get("lon")).and_then(serde_json::Value::as_f64));
    let monthly = proj
        .years
        .first()
        .map(|y| ((y.recurring + y.interest) / 12.0).round())
        .unwrap_or(0.0);
    json!({
        "id": l.id,
        "title": l.title(),
        "municipality": l.municipality,
        "country": l.country_enum().code(),
        "address": l.address,
        "price_eur": l.price_eur,
        "property_type": l.property_type,
        "holding_form": l.holding_form,
        "living_area_m2": l.living_area_m2,
        "plot_area_m2": l.plot_area_m2,
        "year_built": l.year_built,
        "room_count": l.room_count,
        "energy_class": l.energy_class,
        "condition_class": l.condition_class,
        "heating_type": l.heating_type,
        "shore": l.shore,
        "water_body": l.water_body,
        "plot_ownership": l.plot_ownership,
        "water_supply": l.water_supply,
        "sewer_system": l.sewer_system,
        "broadband": l.broadband,
        "roof_year": l.roof_year,
        "pipes_renovated_year": l.pipes_renovated_year,
        "lat": lat,
        "lon": lon,
        "description": l.description,
        "source_url": l.url,
        "gallery": gallery,
        "cost": {
            "monthly_living": monthly,
            "npv_cost": proj.npv_cost,
            "horizon_years": horizon,
            "kiinteistovero_eur_yr": l.kiinteistovero_eur_yr,
            "electricity_eur_yr": l.electricity_eur_yr,
            "cash": true,
        },
        "risk": {
            "score": assessment.score,
            "band": assessment.band.label(),
            "deferred_capex_eur": assessment.deferred_capex_eur,
            "flags": assessment.flags.iter().map(|f| json!({ "label": f.label, "points": f.points, "capex_eur": f.capex_eur })).collect::<Vec<_>>(),
        },
        "reasons": publish_reasons(l, tier, off_spec),
        "off_spec": off_spec,
        "tier": tier,
        "published_at": "",
    })
}

fn assess(detail: &ListingDetail) -> RiskAssessment {
    let near_water = detail
        .dossier
        .as_ref()
        .and_then(|d| d.get("distance_to_water_m"))
        .and_then(|v| v.as_f64())
        .map(|m| m < 150.0)
        .unwrap_or(false);
    risk::assess(&detail.listing.to_risk_input(near_water), 2026)
}

fn cost_summary(p: &Projection) -> serde_json::Value {
    json!({
        "npv_cost": p.npv_cost,
        "equivalent_monthly": p.equivalent_monthly,
        "one_time": p.one_time,
        "total_loan_interest": p.total_loan_interest,
        "loan_principal": p.loan_principal,
        "terminal_equity": p.terminal_equity,
    })
}

fn emit<T: serde::Serialize>(v: &T) -> Result<()> {
    println!("{}", serde_json::to_string(v)?);
    Ok(())
}

fn emit_or<T: serde::Serialize>(v: &T, json: bool, human: &str) {
    if json {
        let _ = emit(v);
    } else {
        println!("{human}");
    }
}

fn ok(json: bool, msg: String) {
    if json {
        println!("{}", json!({ "ok": true, "message": msg }));
    } else {
        println!("{msg}");
    }
}

fn parse_sort(s: &str) -> SortColumn {
    match s {
        "ppm2" => SortColumn::PricePerM2,
        "size" => SortColumn::SizeM2,
        "year" => SortColumn::YearBuilt,
        "dom" => SortColumn::DaysOnMarket,
        "risk" => SortColumn::RiskScore,
        "score" => SortColumn::Score,
        _ => SortColumn::Price,
    }
}

fn parse_heating(s: &str) -> HeatingType {
    let s = s.to_lowercase();
    if s.contains("maa") {
        HeatingType::Maalampo
    } else if s.contains("olj") || s.contains("öljy") {
        HeatingType::Oljy
    } else if s.contains("ivlp") || s.contains("ilmavesi") {
        HeatingType::Ivlp
    } else if s.contains("puu") || s.contains("pelle") {
        HeatingType::Puu
    } else if s.contains("sah") || s.contains("säh") {
        HeatingType::Sahko
    } else {
        HeatingType::Kaukolampo
    }
}

fn parse_repayment(s: &str) -> RepaymentType {
    let s = s.to_lowercase();
    if s.starts_with("tas") {
        RepaymentType::Tasalyhennys
    } else if s.starts_with("kiin") {
        RepaymentType::KiinteaTasaera
    } else {
        RepaymentType::Annuiteetti
    }
}

fn risk_of(l: &Listing) -> u32 {
    risk::assess(&l.to_risk_input(false), 2026).score
}

fn print_list(listings: &[Listing], total: i64) {
    if listings.is_empty() {
        println!("no listings ({total} total)");
        return;
    }
    println!(
        "{:<6} {:<24} {:>10} {:>9} {:>6} {:>5} {:>4} {:>4}",
        "ID", "PLACE", "PRICE", "€/M2", "M2", "YR", "RSK", "DOM"
    );
    for l in listings {
        println!(
            "{:<6} {:<24} {:>10} {:>9} {:>6} {:>5} {:>4} {:>4}",
            l.id,
            trunc(&l.title(), 24),
            money_opt(l.price_eur),
            ppm2_opt(l.effective_ppm2()),
            area_opt(l.living_area_m2),
            int_opt(l.year_built),
            risk_of(l),
            l.days_on_market.map(|d| d.to_string()).unwrap_or_else(|| "—".into()),
        );
    }
    println!("{} of {total} shown", listings.len());
}

fn print_show(detail: &ListingDetail, risk: &RiskAssessment, proj: &Projection) {
    let l = &detail.listing;
    println!("#{}  {}  {}", l.id, l.title(), money_opt(l.price_eur));
    println!(
        "{} · {} · {} · {}",
        str_opt(&l.property_type),
        str_opt(&l.municipality),
        str_opt(&l.holding_form),
        l.status
    );
    println!(
        "area {} · plot {} · {} · {}/m² · {} rooms · built {}",
        area_opt(l.living_area_m2),
        area_opt(l.plot_area_m2),
        str_opt(&l.energy_class),
        ppm2_opt(l.effective_ppm2()),
        num_opt(l.room_count),
        int_opt(l.year_built),
    );
    println!(
        "heating {} · water {} · sewer {} · shore {} · plot {}",
        str_opt(&l.heating_type),
        str_opt(&l.water_supply),
        str_opt(&l.sewer_system),
        str_opt(&l.shore),
        str_opt(&l.plot_ownership),
    );
    println!(
        "cost: NPV {} (~{}/mo) · transfer tax {} · interest {}",
        money(proj.npv_cost),
        money(proj.equivalent_monthly),
        money(proj.one_time.transfer_tax),
        money(proj.total_loan_interest),
    );
    println!("risk: {} ({}) · deferred capex ~{}", risk.score, risk.band.label(), money(risk.deferred_capex_eur));
    if let Some(f) = &l.fairness {
        let bm = f
            .benchmark
            .map(|b| format!(" · area median {}", money(b)))
            .unwrap_or_default();
        println!("fair price: {} ({}){}", f.band, f.confidence, bm);
    }
    for f in &risk.flags {
        println!("  - {}", f.label);
    }
    if let Some(score) = detail.score.as_ref().and_then(|s| s.score) {
        println!("your score: {score}");
    }
    if let Some(note) = &detail.note {
        println!("note: {note}");
    }
    println!("url: {}", l.url);
}

fn print_cost(l: &Listing, p: &Projection, schedule: bool) {
    println!("#{} {}", l.id, l.title());
    println!("net present cost  {}", money(p.npv_cost));
    println!("≈ per month       {}", money(p.equivalent_monthly));
    println!("down payment      {}", money(p.one_time.down_payment));
    println!("transfer tax      {}", money(p.one_time.transfer_tax));
    println!("up-front total    {}", money(p.one_time.total()));
    println!("loan interest     {}", money(p.total_loan_interest));
    println!("terminal equity   {}", money(p.terminal_equity));
    if schedule {
        println!("{:>4} {:>12} {:>12} {:>12}", "YR", "INTEREST", "RECURRING", "TOTAL");
        for y in &p.years {
            println!(
                "{:>4} {:>12} {:>12} {:>12}",
                y.year,
                money(y.interest),
                money(y.recurring),
                money(y.total_nominal)
            );
        }
    }
}

fn print_risk(l: &Listing, r: &RiskAssessment) {
    println!("#{} {} — risk {} ({})", l.id, l.title(), r.score, r.band.label());
    println!("deferred capex ~{}", money(r.deferred_capex_eur));
    for f in &r.flags {
        let capex = if f.capex_eur > 0.0 {
            format!(" (~{})", money(f.capex_eur))
        } else {
            String::new()
        };
        println!("  [{:>2}] {}{}", f.points, f.label, capex);
    }
}

fn print_compare(rows: &[(Listing, u32, f64, f64)]) {
    println!(
        "{:<6} {:<22} {:>10} {:>9} {:>4} {:>11} {:>9}",
        "ID", "PLACE", "PRICE", "€/M2", "RSK", "NPV", "€/MO"
    );
    for (l, score, npv, mo) in rows {
        println!(
            "{:<6} {:<22} {:>10} {:>9} {:>4} {:>11} {:>9}",
            l.id,
            trunc(&l.title(), 22),
            money_opt(l.price_eur),
            ppm2_opt(l.effective_ppm2()),
            score,
            money(*npv),
            money(*mo),
        );
    }
}

fn print_defaults(d: &crate::cost::CostDefaults) {
    println!("varainsiirtovero  kiinteistö {:.1}%  osake {:.1}%", d.transfer_tax_kiinteisto * 100.0, d.transfer_tax_osake * 100.0);
    println!("euribor 12mo      {:.3}%   margin {:.2}%", d.euribor_12m * 100.0, d.mortgage_margin * 100.0);
    println!("lainakatto        {:.0}% / {:.0}% first-home", d.ltv_max * 100.0, d.ltv_first_home * 100.0);
    println!("registration      lainhuuto {}€ · kaupanvahvistus {}€ · kiinnitys {}€", d.lainhuuto_eur as i64, d.kaupanvahvistus_eur as i64, d.kiinnitys_eur as i64);
    println!("kiinteistövero    permanent {:.2}–{:.2}% · general {:.2}–{:.2}% · land {:.2}–{:.2}%",
        d.kvero_building_permanent_min * 100.0, d.kvero_building_permanent_max * 100.0,
        d.kvero_building_general_min * 100.0, d.kvero_building_general_max * 100.0,
        d.kvero_land_min * 100.0, d.kvero_land_max * 100.0);
}

fn trunc(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matching::Scored;

    fn outlier(id: i64, country: &str, price: i64) -> Scored {
        Scored {
            id,
            title: String::new(),
            municipality: None,
            country: country.into(),
            price_eur: Some(price),
            property_type: None,
            url: String::new(),
            score: 0.0,
            npv_cost: 0.0,
            monthly: 0.0,
            monthly_living: 0.0,
            risk: 0,
            pinned: false,
            near_miss: false,
            value_outlier: true,
            off_spec: vec![],
            fairness_ratio: None,
            reasons: vec![],
        }
    }

    #[test]
    fn select_outliers_balances_across_countries() {
        // Finland floods the lane; the round-robin must still surface every market.
        let mut pool: Vec<Scored> = (0..15).map(|i| outlier(i, "FI", 10_000 + i)).collect();
        pool.extend((0..6).map(|i| outlier(100 + i, "SE", 20_000 + i)));
        pool.extend((0..2).map(|i| outlier(200 + i, "NO", 30_000 + i)));
        pool.extend((0..1).map(|i| outlier(300 + i, "IS", 40_000 + i)));
        let picked = select_outliers(pool, 12);
        let countries: std::collections::BTreeSet<_> = picked.iter().map(|p| p.country.clone()).collect();
        assert!(picked.len() <= 12);
        assert!(countries.contains("SE") && countries.contains("NO") && countries.contains("IS"),
            "every non-FI market must be represented, got {countries:?}");
        let fi = picked.iter().filter(|p| p.country == "FI").count();
        assert!(fi <= 6, "Finland must not dominate the balanced lane (got {fi}/12)");
    }

    #[test]
    fn select_outliers_within_a_country_leads_with_the_deepest_steal() {
        let mut a = outlier(1, "FI", 90_000);
        a.fairness_ratio = Some(0.6);
        let mut b = outlier(2, "FI", 50_000);
        b.fairness_ratio = Some(0.3);
        let picked = select_outliers(vec![a, b], 12);
        assert_eq!(picked[0].id, 2, "the steeper discount (lower ratio) leads its country lane");
    }
}
