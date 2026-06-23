//! Agent-native command line. Every subcommand supports `--json` for structured
//! output, and `--help` documents the surface so an LLM can discover and drive
//! it. With no subcommand the binary launches the interactive TUI instead.

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use serde_json::json;

use crate::api::KontuClient;
use crate::app::CostState;
use crate::cost::{HeatingType, Projection, RepaymentType};
use crate::format::{area_opt, int_opt, money, money_opt, num_opt, ppm2_opt, str_opt};
use crate::models::{FilterState, Listing, ListingDetail, SortColumn};
use crate::risk::{self, RiskAssessment};

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
kontu note 8002 \"Lakeside; book a kuntotutkimus.\"\n\n\
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
}

#[derive(Args, Debug)]
pub struct ListArgs {
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
    rooms_min: Option<f64>,
    #[arg(long)]
    year_min: Option<i32>,
    /// Shore: oma_ranta | rantaoikeus | ei_rantaa
    #[arg(long)]
    shore: Option<String>,
    #[arg(long)]
    heating: Option<String>,
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
            municipality: self.municipality,
            property_type: self.property_type,
            holding_form: self.holding,
            price_min: self.price_min,
            price_max: self.price_max,
            m2_min: self.m2_min,
            m2_max: None,
            rooms_min: self.rooms_min,
            year_min: self.year_min,
            shore: self.shore,
            heating_type: self.heating,
            energy_class_max: None,
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
    }
}

pub async fn run(command: Command, client: &KontuClient, json: bool) -> Result<()> {
    match command {
        Command::List(a) => {
            let (filter, sort, desc, limit) = a.into_query();
            let page = client.list_listings(&filter, sort, desc, limit, 0).await?;
            if json {
                emit(&page)?;
            } else {
                print_list(&page.listings, page.total);
            }
        }
        Command::Show { id } => {
            let detail = client.get_listing(id).await?;
            let defaults = client.cost_defaults().await.unwrap_or_default();
            let assessment = assess(&detail);
            let mut cs = CostState::from_defaults(&defaults);
            cs.apply_listing(&detail.listing, &assessment, &defaults);
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
            let defaults = client.cost_defaults().await.unwrap_or_default();
            let assessment = assess(&detail);
            let mut cs = CostState::from_defaults(&defaults);
            cs.apply_listing(&detail.listing, &assessment, &defaults);
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
            let defaults = client.cost_defaults().await.unwrap_or_default();
            let mut rows = Vec::new();
            for id in ids {
                let detail = client.get_listing(id).await?;
                let assessment = assess(&detail);
                let mut cs = CostState::from_defaults(&defaults);
                cs.apply_listing(&detail.listing, &assessment, &defaults);
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
    }
    Ok(())
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
