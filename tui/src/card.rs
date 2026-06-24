//! `kontu card <id>` — render a shareable PNG "ownership one-pager" for a listing:
//! the cover photo plus the facts a buyer (and a careful parent) checks before
//! owning — price, all-in acquisition cost, monthly running cost, property tax,
//! condition, and what recurring obligations the property avoids.

use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::api::KontuClient;
use crate::app::CostState;
use crate::models::{Listing, ListingDetail};
use crate::risk::{self, RiskAssessment};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Fi,
    En,
}

impl Lang {
    pub fn parse(s: &str) -> Lang {
        if s.eq_ignore_ascii_case("en") { Lang::En } else { Lang::Fi }
    }
}

/// Group an integer with spaces, Finnish style: 100000 -> "100 000".
fn thousands(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Minimal base64 (standard alphabet) for embedding the cover photo as a data URI.
fn base64(data: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(A[(n >> 18 & 63) as usize] as char);
        out.push(A[(n >> 12 & 63) as usize] as char);
        out.push(if chunk.len() > 1 { A[(n >> 6 & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { A[(n & 63) as usize] as char } else { '=' });
    }
    out
}

/// Render the card for a listing and return the written PNG path.
pub async fn render_card(
    client: &KontuClient,
    id: i64,
    lang: Lang,
    out: Option<PathBuf>,
) -> Result<PathBuf> {
    let detail = client.get_listing(id).await?;
    let defaults = client.cost_defaults().await.unwrap_or_default();
    let l = &detail.listing;

    let near_water = l
        .shore
        .as_deref()
        .map(|s| s.contains("oma_ranta") || s.contains("rantaoik"))
        .unwrap_or(false);
    let assessment = risk::assess(&l.to_risk_input(near_water), 2026);
    let mut cs = CostState::from_defaults(&defaults);
    cs.apply_listing(l, &assessment, &defaults);
    cs.ltv = 0.0;
    let proj = cs.project(&defaults);

    let cover_bytes = match detail.photos.first() {
        Some(p) => client.photo_bytes(&p.r2_key).await.ok(),
        None => None,
    };

    let svg = build_svg(&detail, &assessment, &proj, cover_bytes.as_deref(), lang);
    let out = out.unwrap_or_else(|| default_out_path(l));
    render_svg_to_png(&svg, &out)?;
    Ok(out)
}

fn default_out_path(l: &Listing) -> PathBuf {
    let muni = l
        .municipality
        .as_deref()
        .unwrap_or("kohde")
        .to_lowercase()
        .replace(|c: char| !c.is_ascii_alphanumeric(), "");
    let home = directories::BaseDirs::new()
        .map(|b| b.home_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(format!("kontu-{muni}-{}.png", l.id))
}

fn render_svg_to_png(svg: &str, out: &std::path::Path) -> Result<()> {
    use resvg::{tiny_skia, usvg};
    let mut opt = usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();
    let tree = usvg::Tree::from_str(svg, &opt).context("parsing generated SVG")?;
    let size = tree.size().to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height())
        .context("allocating pixmap")?;
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
    pixmap.save_png(out).with_context(|| format!("writing {}", out.display()))?;
    Ok(())
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn wrap_text(s: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in s.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

const GREEN: &str = "#21402f";
const GREEN2: &str = "#2d5640";
const CREAM: &str = "#f6f1e6";
const GOLD: &str = "#c79a3e";
const INK: &str = "#2b2b26";
const MUT: &str = "#6b6a60";
const LINE: &str = "#e3dcc9";

fn build_svg(
    detail: &ListingDetail,
    risk: &RiskAssessment,
    proj: &crate::cost::Projection,
    cover: Option<&[u8]>,
    lang: Lang,
) -> String {
    let l = &detail.listing;
    let fi = lang == Lang::Fi;
    let t = |a: &str, b: &str| -> String { if fi { a } else { b }.to_string() };

    let (w, h, ph) = (1080i64, 2010i64, 560i64);
    let img = cover.map(|b| format!("data:image/jpeg;base64,{}", base64(b))).unwrap_or_default();

    let price = l.price_eur.map(|p| format!("{} €", thousands(p))).unwrap_or_else(|| "—".into());
    let monthly = (proj.years.first().map(|y| y.recurring).unwrap_or(0.0) / 12.0).round() as i64;
    let acq = (l.price_eur.unwrap_or(0) as f64
        + proj.one_time.transfer_tax
        + proj.one_time.lainhuuto
        + proj.one_time.kaupanvahvistus
        + proj.one_time.inspection)
        .round() as i64;
    let kvero = l.kiinteistovero_eur_yr;
    let plot_ha = l.plot_area_m2.map(|m| m / 10000.0);
    let energy = l.energy_class.clone().unwrap_or_else(|| "–".into());
    let kunto = l.condition_class.as_deref().map(cap).unwrap_or_else(|| t("ei arviota", "n/a"));
    let mat = l.frame_material.clone().unwrap_or_else(|| "–".into());
    let shore_txt = match l.shore.as_deref() {
        Some(s) if s.contains("oma_ranta") => t("oma ranta", "own shore"),
        Some(s) if s.contains("rantaoik") => t("rantaoikeus", "shore right"),
        _ => t("–", "–"),
    };
    let muni = l.municipality.clone().unwrap_or_default();
    let addr = l.address.clone().unwrap_or_default();
    let term = proj.terminal_equity.round() as i64;

    // dynamic "why" bullets
    let mut bullets: Vec<String> = Vec::new();
    bullets.push(t(
        "Ostetaan kokonaan käteisellä — ei asuntolainaa, ei velkaa, ei pankkia.",
        "Bought outright in cash — no mortgage, no debt, no bank.",
    ));
    if matches!(l.condition_class.as_deref(), Some("hyvä") | Some("erinomainen")) {
        let mut reno = Vec::new();
        if let Some(y) = l.roof_year {
            reno.push(t(&format!("vesikatto {y}"), &format!("roof {y}")));
        }
        if let Some(y) = l.pipes_renovated_year {
            reno.push(t(&format!("putket {y}"), &format!("pipes {y}")));
        }
        let r = if reno.is_empty() { String::new() } else { format!(" ({})", reno.join(", ")) };
        bullets.push(t(
            &format!("Muuttovalmis: virallinen kuntoluokka {kunto}{r} — ei remonttikohde."),
            &format!("Move-in ready: official condition {kunto}{r} — not a fixer-upper."),
        ));
    }
    if let (Some(ha), true) = (plot_ha, shore_txt.contains("ranta") || !fi) {
        bullets.push(t(
            &format!("{} hehtaarin oma rantatontti — harvinaista, arvonsa säilyttävää maata.", fmt_ha(ha)),
            &format!("{:.2} ha of own lakefront — scarce land that holds its value.", ha),
        ));
    }
    bullets.push(t(
        &format!("Asumiskulut noin {monthly} €/kk lämmityksineen — koti, joka ei rasita taloutta."),
        &format!("About {monthly} EUR/mo to run, fully heated — a home that never strains the budget."),
    ));

    // "what you don't pay"
    let mut no_pay = vec![t("ei asuntolainaa", "no mortgage")];
    if !l.holding_form.as_deref().unwrap_or("").contains("osake") {
        no_pay.push(t("ei vastiketta", "no service charge"));
    }
    if l.plot_ownership.as_deref().map(|o| o.contains("oma")).unwrap_or(false) {
        no_pay.push(t("ei tonttivuokraa", "no ground rent"));
    }

    let kvero_txt = kvero
        .map(|v| format!("{v} €/v"))
        .unwrap_or_else(|| t("ei tiedossa", "n/a"));
    let reno_cell = {
        let r = l.roof_year.or(l.pipes_renovated_year);
        match r {
            Some(y) => format!("{} · {} {y}", l.year_built.map(|x| x.to_string()).unwrap_or_else(|| "–".into()),
                if l.roof_year.is_some() { t("katto", "roof") } else { t("putket", "pipes") }),
            None => l.year_built.map(|x| x.to_string()).unwrap_or_else(|| "–".into()),
        }
    };

    let mut s: Vec<String> = Vec::new();
    s.push(format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {w} {h}">"##,
        w * 2, h * 2
    ));
    s.push(format!(r##"<defs><linearGradient id="ph" x1="0" y1="0" x2="0" y2="1"><stop offset="0.45" stop-color="#000" stop-opacity="0"/><stop offset="1" stop-color="#000" stop-opacity="0.72"/></linearGradient><clipPath id="pc"><rect width="{w}" height="{ph}"/></clipPath></defs>"##));
    s.push(format!(r##"<rect width="{w}" height="{h}" fill="{CREAM}"/>"##));
    if !img.is_empty() {
        s.push(format!(r##"<g clip-path="url(#pc)"><image width="{w}" height="{ph}" preserveAspectRatio="xMidYMid slice" href="{img}"/><rect width="{w}" height="{ph}" fill="url(#ph)"/></g>"##));
    } else {
        s.push(format!(r##"<rect width="{w}" height="{ph}" fill="{GREEN}"/>"##));
    }
    let tag = t("luonnonrauha & oma ranta", "lakeside ownership").to_uppercase();
    s.push(format!(r##"<rect x="48" y="40" rx="6" width="{}" height="42" fill="{GOLD}"/>"##, 40 + (tag.chars().count() as i64) * 13));
    s.push(format!(r##"<text x="62" y="69" font-family="Liberation Sans" font-size="20" font-weight="bold" fill="#1c1c18" letter-spacing="1">{}</text>"##, esc(&tag)));
    s.push(format!(r##"<text x="48" y="{}" font-family="Liberation Serif" font-size="62" font-weight="bold" fill="#fff">{}</text>"##, ph - 70, esc(&muni)));
    s.push(format!(r##"<text x="50" y="{}" font-family="Liberation Sans" font-size="27" fill="#f2eede">{} · {}</text>"##, ph - 28, esc(&addr), esc(&shore_txt)));
    s.push(format!(r##"<text x="{}" y="{}" text-anchor="end" font-family="Liberation Serif" font-size="56" font-weight="bold" fill="#fff">{}</text>"##, w - 48, ph - 32, esc(&price)));
    s.push(format!(r##"<text x="{}" y="{}" text-anchor="end" font-family="Liberation Sans" font-size="20" font-weight="bold" fill="{GOLD}">{}</text>"##, w - 48, ph - 78, esc(&t("KÄTEISKAUPPA — EI ASUNTOLAINAA", "PAID IN FULL — NO MORTGAGE"))));

    let mut y = ph + 58;
    let mtxt = l.living_area_m2.map(|m| format!("{} m²", m as i64)).unwrap_or_default();
    let yr = l.year_built.map(|x| x.to_string()).unwrap_or_default();
    let stats = format!("{mtxt}  ·  {}  ·  {} {yr}", plot_ha.map(|h| format!("{} ha", fmt_ha(h))).unwrap_or_default(), t("rakennettu", "built"));
    s.push(format!(r##"<text x="48" y="{y}" font-family="Liberation Sans" font-size="24" fill="{MUT}">{}</text>"##, esc(&stats)));
    y += 54;
    s.push(format!(r##"<text x="48" y="{y}" font-family="Liberation Serif" font-size="33" font-weight="bold" fill="{GREEN}">{}</text>"##, esc(&t("Miksi tämä on järkevä ostos", "Why this is a sound buy"))));
    y += 14;
    for b in &bullets {
        y += 28;
        s.push(format!(r##"<circle cx="60" cy="{}" r="7" fill="{GOLD}"/>"##, y - 6));
        for ln in wrap_text(b, 60) {
            s.push(format!(r##"<text x="86" y="{y}" font-family="Liberation Sans" font-size="25" fill="{INK}">{}</text>"##, esc(&ln)));
            y += 33;
        }
        y += 6;
    }

    // ownership costs section
    y += 18;
    s.push(format!(r##"<text x="48" y="{y}" font-family="Liberation Sans" font-size="20" font-weight="bold" fill="{GOLD}" letter-spacing="1">{}</text>"##, esc(&t("OMISTUKSEN KULUT", "COST OF OWNERSHIP"))));
    y += 16;
    let cells1 = [
        (t("Kauppahinta", "Purchase price"), price.clone()),
        (t("Hankintakustannus", "All-in to acquire"), format!("~{} €", thousands(acq))),
        (t("Asumiskulut / kk", "To run / mo"), format!("~{monthly} €/kk")),
        (t("Kiinteistövero", "Property tax"), kvero_txt.clone()),
    ];
    y = grid(&mut s, y, &cells1, 2);

    // what you don't pay
    y += 8;
    s.push(format!(r##"<rect x="48" y="{y}" rx="10" width="{}" height="66" fill="#eef2e8" stroke="{LINE}"/>"##, w - 96));
    s.push(format!(r##"<text x="70" y="{}" font-family="Liberation Sans" font-size="23" fill="{GREEN2}"><tspan font-weight="bold">{}:</tspan>  {}</text>"##, y + 42, esc(&t("Ei toistuvia maksuja", "No recurring obligations")), esc(&no_pay.join("  ·  "))));
    y += 66 + 24;

    // property facts
    s.push(format!(r##"<text x="48" y="{y}" font-family="Liberation Sans" font-size="20" font-weight="bold" fill="{GOLD}" letter-spacing="1">{}</text>"##, esc(&t("KOHTEEN TIEDOT", "THE PROPERTY"))));
    y += 16;
    let cells2 = [
        (t("Kuntoluokka", "Condition"), format!("{kunto} · {} {}/100", t("riski", "risk"), risk.score)),
        (t("Energialuokka", "Energy class"), energy.clone()),
        (t("Oma tontti", "Own plot"), format!("{} + {}", plot_ha.map(|h| format!("{} ha", fmt_ha(h))).unwrap_or_default(), shore_txt)),
        (t("Rakennettu / remontti", "Built / renovated"), reno_cell.clone()),
        (t("Rakennusmateriaali", "Build material"), mat.clone()),
        (t("Lämmitys", "Heating"), l.heating_type.clone().unwrap_or_else(|| "–".into())),
    ];
    y = grid(&mut s, y, &cells2, 2);

    // value banner
    y += 14;
    s.push(format!(r##"<rect x="48" y="{y}" rx="12" width="{}" height="118" fill="{GREEN}"/>"##, w - 96));
    s.push(format!(r##"<text x="74" y="{}" font-family="Liberation Sans" font-size="20" fill="{GOLD}" letter-spacing="1">{}</text>"##, y + 42, esc(&t("OMAISUUTTA, EI KULUA", "AN ASSET, NOT AN EXPENSE"))));
    let vline = t(
        &format!("Arvioitu arvo 20 vuoden kuluttua noin {} € — enemmän kuin maksettu {}. Raha ostaa pysyvää, arvonsa säilyttävää omaisuutta.", thousands(term), price),
        &format!("Modelled value in 20 years about {} € — above the {} paid. The money buys a lasting, appreciating asset.", thousands(term), price),
    );
    for (i, ln) in wrap_text(&vline, 72).iter().enumerate() {
        s.push(format!(r##"<text x="74" y="{}" font-family="Liberation Sans" font-size="23" fill="#f3efe2">{}</text>"##, y + 78 + i as i64 * 30, esc(ln)));
    }
    y += 118 + 40;
    s.push(format!(r##"<text x="48" y="{y}" font-family="Liberation Serif" font-size="23" font-style="italic" fill="{MUT}">{}</text>"##, esc(&t("Koottu kontulla — luvut virallisesta myynti-ilmoituksesta ja kustannusmalleista.", "Built with kontu — figures from the official listing & local cost models."))));
    s.push("</svg>".into());
    s.join("\n")
}

fn fmt_ha(ha: f64) -> String {
    format!("{ha:.2}").replace('.', ",")
}

/// Render a row-major grid of fact cells, returning the y after the last row.
fn grid(s: &mut Vec<String>, y0: i64, cells: &[(String, String)], cols: i64) -> i64 {
    let (w, gh, gap) = (1080i64, 92i64, 24i64);
    let gw = (w - 96 - gap * (cols - 1)) / cols;
    for (i, (k, v)) in cells.iter().enumerate() {
        let i = i as i64;
        let (col, row) = (i % cols, i / cols);
        let x = 48 + col * (gw + gap);
        let ry = y0 + row * (gh + 16);
        s.push(format!(r##"<rect x="{x}" y="{ry}" rx="10" width="{gw}" height="{gh}" fill="#fff" stroke="{LINE}"/>"##));
        s.push(format!(r##"<text x="{}" y="{}" font-family="Liberation Sans" font-size="19" fill="{MUT}" letter-spacing="0.5">{}</text>"##, x + 22, ry + 34, esc(&k.to_uppercase())));
        s.push(format!(r##"<text x="{}" y="{}" font-family="Liberation Serif" font-size="26" font-weight="bold" fill="{GREEN2}">{}</text>"##, x + 22, ry + 72, esc(v)));
    }
    let rows = (cells.len() as i64 + cols - 1) / cols;
    y0 + rows * (gh + 16)
}
