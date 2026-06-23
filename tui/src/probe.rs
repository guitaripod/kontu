//! Headless end-to-end probe (`kontu --probe`): exercises the same client the
//! TUI uses against a live Worker and runs the cost engine on real listings.
//! Used for E2E verification where a TTY isn't available.

use anyhow::Result;

use crate::api::KontuClient;
use crate::app::CostState;
use crate::format::{money, money_opt};
use crate::models::{FilterState, SortColumn};

pub async fn run(client: &KontuClient) -> Result<()> {
    println!("── kontu E2E probe ──");

    let healthy = client.health().await?;
    println!("health           : {}", if healthy { "ok" } else { "DOWN" });

    let defaults = client.cost_defaults().await?;
    println!(
        "cost-defaults    : varainsiirtovero kiinteistö {:.1}% · osake {:.1}% · Euribor {:.3}% · lainhuuto {}€",
        defaults.transfer_tax_kiinteisto * 100.0,
        defaults.transfer_tax_osake * 100.0,
        defaults.euribor_12m * 100.0,
        defaults.lainhuuto_eur as i64,
    );

    let page = client
        .list_listings(&FilterState::default(), SortColumn::Price, false, 100, 0)
        .await?;
    println!("listings         : {} (total {})", page.listings.len(), page.total);
    for l in page.listings.iter().take(6) {
        println!(
            "   #{:<5} {:<22} {:>10}  {}",
            l.id,
            l.title(),
            money_opt(l.price_eur),
            l.property_type.clone().unwrap_or_default(),
        );
    }

    let mut outk = FilterState::default();
    outk.municipality = Some("Outokumpu".into());
    let filtered = client.list_listings(&outk, SortColumn::Price, false, 100, 0).await?;
    println!("filter Outokumpu : {} listings", filtered.listings.len());

    let mut shore = FilterState::default();
    shore.shore = Some("oma_ranta".into());
    let shoref = client.list_listings(&shore, SortColumn::PricePerM2, true, 100, 0).await?;
    println!("filter oma_ranta : {} listings", shoref.listings.len());

    if let Some(first) = page.listings.first() {
        let detail = client.get_listing(first.id).await?;
        println!(
            "detail #{}      : {} photos · {} history events · dossier {}",
            detail.listing.id,
            detail.photos.len(),
            detail.events.len(),
            if detail.dossier.is_some() { "present" } else { "none" },
        );

        let risk = crate::risk::assess(&detail.listing.to_risk_input(false), 2026);
        let mut cost = CostState::from_defaults(&defaults);
        cost.apply_listing(&detail.listing, &risk, &defaults);
        let proj = cost.project(&defaults);
        println!(
            "cost model #{}   : NPV {} (~{}/mo) · transfer tax {} · risk {} ({})",
            first.id,
            money(proj.npv_cost),
            money(proj.equivalent_monthly),
            money(proj.one_time.transfer_tax),
            risk.score,
            risk.band.label(),
        );
    }

    println!("── all probes succeeded ──");
    Ok(())
}
