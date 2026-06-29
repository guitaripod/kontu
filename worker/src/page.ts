/**
 * Public listing page (`GET /h/:id`) rendered from a snapshot the kontu CLI
 * publishes. The cost/risk numbers are computed in the Rust engine and posted
 * here verbatim, so the page never re-implements the models. Images are hotlinked
 * straight from the portal CDN (no R2), and the page is `noindex` — shareable by
 * link, not a public listings site.
 */

export interface PublishedPayload {
  id: number;
  title: string;
  municipality: string | null;
  address: string | null;
  price_eur: number | null;
  property_type: string | null;
  holding_form: string | null;
  living_area_m2: number | null;
  plot_area_m2: number | null;
  year_built: number | null;
  room_count: number | null;
  energy_class: string | null;
  condition_class: string | null;
  heating_type: string | null;
  shore: string | null;
  water_body: string | null;
  plot_ownership: string | null;
  water_supply?: string | null;
  sewer_system?: string | null;
  broadband?: string | null;
  roof_year?: number | null;
  pipes_renovated_year?: number | null;
  lat: number | null;
  lon: number | null;
  description: string | null;
  source_url: string;
  gallery: string[];
  cost: {
    monthly_living: number;
    npv_cost: number;
    horizon_years: number;
    kiinteistovero_eur_yr: number | null;
    electricity_eur_yr: number | null;
    cash: boolean;
  };
  risk: {
    score: number;
    band: string;
    deferred_capex_eur: number;
    flags: { label: string; points: number; capex_eur: number }[];
  };
  reasons: string[];
  /** Required preferences this off-spec value outlier misses (e.g. "Ei omaa järvenrantaa"). */
  off_spec?: string[];
  tier: "gate" | "near_miss" | "pin" | "outlier";
  published_at: string;
  bug_pressure?: {
    mosquito: { score: number; band: string };
    blackfly: { score: number; band: string };
    basis: {
      mire_pct: number;
      mire_source: string;
      lake_pct: number;
      watercourse_km: number;
      radius_km: number;
      latitude: number;
    };
    source: string;
    partial: boolean;
  } | null;
}

const esc = (s: string): string =>
  s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!);

const thousands = (n: number): string => Math.round(n).toString().replace(/\B(?=(\d{3})+(?!\d))/g, " ");

const eur = (n: number | null | undefined): string => (n == null ? "—" : `${thousands(n)} €`);

/** "1,16 ha" for big plots, else "700 m²" — Finnish comma decimals. A value under
 *  ~30 m² for a property plot is a parse glitch, not a real plot → treat as unknown. */
function area(m2: number | null): string {
  if (m2 == null || m2 < 30) return "—";
  if (m2 >= 10000) return `${(m2 / 10000).toFixed(2).replace(".", ",")} ha`;
  return `${thousands(m2)} m²`;
}

/** The risk model emits English flag labels (for the agent/CLI); translate the
 *  known ones to Finnish for the public page. Keyword-matched so minor model
 *  wording changes don't silently fall back to English. */
function riskFi(label: string): string {
  const has = (s: string) => label.toLowerCase().includes(s.toLowerCase());
  if (label.startsWith("Risk structure:")) return "Riskirakenne:" + label.slice("Risk structure:".length);
  if (has("Valesokkeli")) return "Valesokkeli (1960–1990) — vaatii rakenneavauksin tehtävän kuntotutkimuksen";
  if (has("asbestos")) return "Ennen 1994 rakennettu — asbestiriski (haitta-ainekartoitus ennen remonttia)";
  if (has("Construction-era")) return "Rakennusajan (1960–1985) riskirakenteet todennäköisiä";
  if (has("Putkiremontti overdue")) return "Putkiremontti yli aikataulun (yli 40 v, ei tietoa uusinnasta)";
  if (has("Putkiremontti approaching")) return "Putkiremontti lähestyy (putket yli 30 v)";
  if (has("Foundation drains")) return "Salaojat todennäköisesti uusittava";
  if (has("Roof past")) return "Katto ylittänyt käyttöikänsä";
  if (has("Roof age unknown")) return "Katon ikä tuntematon ikääntyvässä talossa";
  if (has("Oil heating")) return "Öljylämmitys — poistuva, korkeat käyttökulut (vaihto suositeltavaa)";
  if (has("Jätevesi upgrade")) return "Jätevesijärjestelmän päivitys todennäköinen (lähellä vesistöä)";
  if (has("Basic jätevesi")) return "Perusjätevesijärjestelmä — tarkista asetuksen 157/2017 vaatimukset";
  if (has("rated poor")) return "Kuntoluokka huono";
  if (has("rated fair")) return "Kuntoluokka välttävä";
  if (has("No condition inspection")) return "Ei kuntotarkastusta tiedossa";
  return label;
}

/** Risk band label → Finnish (the model emits low/moderate/high/severe). */
function bandFi(band: string): string {
  const m: Record<string, string> = { low: "matala", moderate: "kohtalainen", high: "korkea", severe: "vakava" };
  return m[band.toLowerCase()] ?? band;
}

const dec = (n: number): string => String(n).replace(".", ",");

/** Soft, informational bug-pressure card (hyttyset / mäkärät) from open SYKE
 *  geodata. Never gates — purely a comfort signal. Omitted when unavailable. */
function renderBugPressure(bp: PublishedPayload["bug_pressure"]): string {
  if (!bp) return "";
  const b = bp.basis;
  const row = (label: string, ix: { band: string }, tip: string): string =>
    `<div class="bugrow"><span class="buglabel">${label} ${info(tip)}</span><span class="bugband b-${esc(ix.band)}">${esc(ix.band)}</span></div>`;
  return `<section class="card"><h2>Hyttyset &amp; mäkärät ${info(
    "Pehmeä, suuntaa-antava arvio avoimesta paikkatiedosta — EI vaikuta laatuseulan validointiin. (Aliarvostettujen löytöjen listalle pääsee vain matalan hyttys- JA mäkäräpaineen kohteita; kohtalainenkin karsii.) Hyttyskausi painottuu kesä–heinäkuuhun ja vaihtelee vuosittain sään mukaan.",
  )}</h2>
  ${row(
    "Hyttyset (seisova vesi)",
    bp.mosquito,
    `Hyttyset lisääntyvät seisovassa vedessä — soissa, kosteikoissa ja matalilla rannoilla. Soiden osuus arvioitu lähteestä ${esc(
      b.mire_source,
    )} 2 km alueelta, lisäksi järviala.`,
  )}
  ${row(
    "Mäkärät (virtaava vesi)",
    bp.blackfly,
    `Mäkärät lisääntyvät virtaavassa, hapekkaassa vedessä — puroissa ja joissa. Arvioitu SYKE:n uomaverkostosta ${dec(b.radius_km)} km säteellä. Järvenranta (ei jokirantaa) pitää tämän tyypillisesti matalana.`,
  )}
  <p class="bugbasis">Perusteena 2 km alueelta: soita &amp; kosteikkoja ${dec(b.mire_pct)} % (${esc(
    b.mire_source,
  )}), järveä ${dec(b.lake_pct)} %; virtaavaa vettä ${dec(b.watercourse_km)} km (${dec(
    b.radius_km,
  )} km säteellä).<br>Lähde: ${esc(bp.source)}.${
    bp.partial ? " Mittaus osittainen — toinen lähde ei juuri nyt vastannut." : ""
  }</p>
  </section>`;
}

/** matala/kohtalainen/korkea → a short card adjective ("vähän hyttysiä"). */
function bugWord(band: string): string {
  return band === "matala" ? "vähän" : band === "korkea" ? "paljon" : "kohtalaisesti";
}

/** Combined mosquito+blackfly pressure (0 = none). Unknown ≈ moderate so a
 *  data gap neither flatters nor unduly buries a listing. */
function bugScoreOf(bp: PublishedPayload["bug_pressure"]): number {
  return bp ? bp.mosquito.score + bp.blackfly.score : 0.36;
}

/** For an off-spec "find" the buyer wants essentially NO bugs — even a modest
 *  (kohtalainen) reading on either axis disqualifies it; only low (matala) hyttys- AND
 *  mäkäräpaine survives. Unknown pressure is kept (a data gap can't confirm bugs). */
function tooBuggy(bp: PublishedPayload["bug_pressure"]): boolean {
  return bp != null && (bp.mosquito.band !== "matala" || bp.blackfly.band !== "matala");
}

/** Own lake (järvi) shore — mirrors the ranker's `own_lake_shore`: owned shore on a
 *  water body that isn't a river/pond/sea (unknown body counts as a lake). */
function hasLakeShore(p: PublishedPayload): boolean {
  if (p.shore !== "oma_ranta") return false;
  const w = (p.water_body ?? "").toLowerCase();
  return !(w.includes("joki") || w.includes("lampi") || w.includes("meri"));
}

/** Compact mosquito + blackfly chips for a listing card. */
function bugChips(bp: PublishedPayload["bug_pressure"]): string {
  if (!bp) return "";
  return (
    `<span class="bug b-${esc(bp.mosquito.band)}" title="hyttyset">🦟 ${bugWord(bp.mosquito.band)}</span>` +
    `<span class="bug b-${esc(bp.blackfly.band)}" title="mäkärät">🪰 ${bugWord(bp.blackfly.band)}</span>`
  );
}

/** Critical facts the listing prose buries — surfaced structured-field-first, then
 *  mined from the description. Each returns null when genuinely unknown (the page
 *  then omits the row rather than showing a misleading blank). */
const lc = (p: PublishedPayload): string => (p.description ?? "").toLowerCase();

function ppm2(p: PublishedPayload): string | null {
  if (p.price_eur == null || !p.living_area_m2) return null;
  return `${thousands(p.price_eur / p.living_area_m2)} €/m²`;
}
function materiaaliFi(p: PublishedPayload): string | null {
  const d = lc(p);
  if (/hirsi|hirret|hirrest|hirsirakent/.test(d)) return "Hirsi";
  if (/tiili|tiilist|tiiliverho/.test(d)) return "Tiili";
  if (/element|betoni|kivital|kivirakent/.test(d)) return "Kivi / betoni";
  if (/puurunko|puutalo|puurakente|lautaverho|rintamamies/.test(d)) return "Puu";
  return null;
}
function remontitFi(p: PublishedPayload): string | null {
  const parts: string[] = [];
  if (p.roof_year) parts.push(`katto ${p.roof_year}`);
  if (p.pipes_renovated_year) parts.push(`putket ${p.pipes_renovated_year}`);
  return parts.length ? parts.join(" · ") : null;
}
function netFi(p: PublishedPayload): string | null {
  const d = lc(p);
  const s = (p.broadband ?? "").toLowerCase();
  if (s.includes("kuitu") || /valokuit|kuituyht|kuituliit|kuituun/.test(d)) return "Valokuitu";
  if (s.includes("laajak") || /laajakaista|adsl|\b4g\b|\b5g\b/.test(d)) return "Laajakaista";
  return p.broadband || null;
}
function vesiFi(p: PublishedPayload): string | null {
  const d = lc(p);
  const s = (p.water_supply ?? "").toLowerCase();
  if (s.includes("kunnal") || /kunnallis\w* vesi|kunnan vesi|kaupungin vesi|vesijohto|vesiosuuskun|kunnallistek/.test(d)) return "Kunnallinen / vesijohto";
  if (s.includes("pora") || /porakaivo/.test(d)) return "Porakaivo";
  if (s.includes("kaivo") || /rengaskaivo|oma kaivo|kaivovesi/.test(d)) return "Kaivo";
  if (/kantovesi/.test(d)) return "Kantovesi";
  return p.water_supply || null;
}
function jatevesiFi(p: PublishedPayload): string | null {
  const d = lc(p);
  const s = (p.sewer_system ?? "").toLowerCase();
  if (s.includes("kunnal") || /kunnallis\w* viemär|kunnan viemär|kaupungin viemär|viemäriverkos|viemäri\b/.test(d)) return "Kunnallinen viemäri";
  if (/panospuhdistamo|pienpuhdistamo|maapuhdistamo|imeytyskent|maasuodatus/.test(d)) return "Oma puhdistamo";
  if (s.includes("umpi") || /umpisäili|umpisaili|umpikaivo/.test(d)) return "Umpisäiliö";
  if (s.includes("saostus") || /saostuskaivo|saostus|kolmiosa/.test(d)) return "Saostuskaivo";
  if (/kuivakäymäl|ulkohuussi|huussi|kompostoiva wc|kompostikäymäl/.test(d)) return "Kuivakäymälä";
  return p.sewer_system || null;
}
function evFi(p: PublishedPayload): string | null {
  const d = lc(p);
  if (/sähköaut|latauspist|latausval|ev-lat|3x25|3 x 25|kolmivaih|3-vaih|3 vaih|63a|35a/.test(d)) return "Latausvalmius";
  if (/autotalli|autokatos|autolämmit|lämpötolppa|tolppapaik|lämmityspist/.test(d)) return "Mahdollinen (autotalli / tolppa)";
  return null;
}
function rantaviivaFi(p: PublishedPayload): string | null {
  const d = p.description ?? "";
  const m =
    d.match(/rantaviiva\w*\D{0,14}?(\d{2,4})\s*(?:m\b|metri)/i) ||
    d.match(/(\d{2,4})\s*(?:m\b|metri\w*)\s+(?:omaa\s+)?rantaviiva/i) ||
    d.match(/omaa\s+rantaa\D{0,10}?(\d{2,4})\s*(?:m\b|metri)/i);
  return m ? `~${m[1]} m` : null;
}
function tieFi(p: PublishedPayload): string | null {
  const d = lc(p);
  if (/ei tieyhte|ei tietä perille|vain veneell/.test(d)) return "Ei tietä perille";
  if (/ympärivuotis\w* tie|tie perille|kestopäällyst|tie pihaan|hyvät kulkuyht|hyvä tieyhte/.test(d)) return "Tie perille";
  if (/yksityistie|tiekunta|tieoikeus/.test(d)) return "Yksityistie";
  return null;
}
function palvelutFi(p: PublishedPayload): string | null {
  const d = p.description ?? "";
  const m =
    d.match(/(\d{1,3})\s*km\D{0,28}?(?:keskusta|palvelu|kaup|kylä)/i) ||
    d.match(/(?:keskusta\w*|palvelu\w*)\D{0,18}?(\d{1,3})\s*km/i);
  return m ? `~${m[1]} km palveluihin` : null;
}
function saunaFi(p: PublishedPayload): string | null {
  const d = lc(p);
  if (/rantasauna/.test(d)) return "Rantasauna";
  if (/sauna|kiuas|löyly|savusauna/.test(d)) return "Sauna";
  return null;
}
function naapuritFi(p: PublishedPayload): string | null {
  const d = p.description ?? "";
  const m = d.match(/lähimp\w*\s+naapuri\w*\D{0,18}?(\d{2,4})\s*(m\b|metri\w*|km)/i);
  if (m) return `Lähin naapuri ~${m[1]} ${/^k/i.test(m[2] ?? "") ? "km" : "m"}`;
  const dl = d.toLowerCase();
  if (/haja-asutus|ei naapur|naapureita ei|syrjäss|luonnonrauha|näköest|ei läpikulku|oma rauha|rauhallis/.test(dl)) return "Rauhallinen, ei lähinaapureita";
  if (/keskeisel|keskustass|taajamass|kerrostal/.test(dl)) return "Taajama-alue";
  if (p.plot_area_m2 != null && p.plot_area_m2 >= 10000) return "Väljä, iso oma tontti";
  if (p.plot_area_m2 != null && p.plot_area_m2 >= 3000) return "Väljä tontti";
  return null;
}

function info(tip?: string): string {
  return tip ? ` <button type="button" class="info" data-tip="${esc(tip)}" aria-label="Selitä">i</button>` : "";
}

function fact(label: string, value: string, tip?: string): string {
  if (!value || value === "—") return "";
  return `<div class="fact"><dt>${esc(label)}${info(tip)}</dt><dd>${esc(value)}</dd></div>`;
}

/** Shareable index of every published listing — one stable URL that updates as
 *  the published set changes. Cards link to each `/h/:id`. */
export function renderIndexPage(
  items: PublishedPayload[],
  origin: string,
  market?: { scanned: number; municipalities: number; unread?: number; updated: number | null },
): string {
  const gate = items.filter((p) => p.tier === "gate").sort((a, b) => (a.price_eur ?? 9e9) - (b.price_eur ?? 9e9));
  const almost = items
    .filter((p) => p.tier === "near_miss" || p.tier === "pin")
    .sort(
      (a, b) =>
        (a.tier === "pin" ? 0 : 1) - (b.tier === "pin" ? 0 : 1) ||
        a.risk.score - b.risk.score ||
        (a.price_eur ?? 9e9) - (b.price_eur ?? 9e9),
    );
  // These off-spec finds must EARN their place: secluded (gated in the ranker) and
  // essentially bug-free — drop anything above a low bug reading, then lead with the
  // least buggy. Applies to every outlier, lake or not: a buggy find is not wanted.
  const outliers = items
    .filter((p) => p.tier === "outlier" && !tooBuggy(p.bug_pressure))
    .sort(
      (a, b) => bugScoreOf(a.bug_pressure) - bugScoreOf(b.bug_pressure) || (a.price_eur ?? 9e9) - (b.price_eur ?? 9e9),
    )
    .slice(0, 12);
  const cardOf = (p: PublishedPayload): string => {
    const cover = p.gallery[0] ?? "";
    const place = p.municipality ?? p.title ?? `#${p.id}`;
    const monthly = Math.round(p.cost.monthly_living);
    const isGate = p.tier === "gate";
    const tag = isGate
      ? ""
      : p.tier === "pin"
        ? `<span class="tag pin">★ Suosikki</span>`
        : p.tier === "outlier"
          ? `<span class="tag outlier">Löytö</span>`
          : `<span class="tag near">Melkein</span>`;
    const facts = [
      p.living_area_m2 != null ? `${thousands(p.living_area_m2)} m²` : "",
      area(p.plot_area_m2),
      p.year_built != null ? String(p.year_built) : "",
      p.condition_class ?? "",
    ]
      .filter((x) => x && x !== "—")
      .map(esc)
      .join(" · ");
    // For an off-spec value outlier, lead with what it gives in return for the missing
    // lake: seclusion (gated in the ranker) + low bug pressure. Caveat trails.
    const headline = hasLakeShore(p)
      ? `<span class="bug priv">🌊 Oma ranta</span>`
      : `<span class="bug priv">🌲 Rauhallinen</span>`;
    const merits = p.tier === "outlier" ? `<div class="merits">${headline}${bugChips(p.bug_pressure)}</div>` : "";
    const offspec =
      p.tier === "outlier" && p.off_spec?.length
        ? `<div class="offspec">${p.off_spec.map(esc).join(" · ")}</div>`
        : "";
    return `<a class="tile${isGate ? "" : " almost"}" href="${origin}/kontu/${p.id}">
        <div class="thumb">${cover ? `<img src="${esc(cover)}" loading="${isGate ? "eager" : "lazy"}" decoding="async" alt="" referrerpolicy="no-referrer">` : ""}${tag}</div>
        <div class="meta"><div class="place">${esc(place)}</div>
          <div class="row"><span class="p">${eur(p.price_eur)}</span><span class="m">${thousands(monthly)} €/kk · riski ${p.risk.score}</span></div>
          ${merits}<div class="facts2">${facts}</div>${offspec}
        </div></a>`;
  };
  const gateCards = gate.map(cardOf).join("");
  const almostCards = almost.map(cardOf).join("");
  const outlierCards = outliers.map(cardOf).join("");
  const n = gate.length;
  const almostN = almost.length;
  const outlierN = outliers.length;
  const scanned = market?.scanned ?? n;
  const munis = market?.municipalities ?? 0;
  const unread = market?.unread ?? 0;
  const updated =
    market?.updated != null
      ? new Intl.DateTimeFormat("fi-FI", { timeZone: "Europe/Helsinki", day: "numeric", month: "numeric", year: "numeric" }).format(
          new Date(market.updated * 1000),
        )
      : null;
  return `<!doctype html><html lang="fi"><head>
<meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex, nofollow"><title>Kontu — kohteet (${n})</title>
<meta property="og:title" content="Kontu — ${n} validoitua kohdetta"><meta property="og:description" content="Laatuseula kävi läpi ${scanned} ranta-asuntoilmoitusta ${munis} kunnasta — ${n} läpäisi. Kustannukset ja ostajan riski mallinnettu.">
<meta property="og:type" content="website"><meta name="twitter:card" content="summary">
<link rel="icon" type="image/svg+xml" href="/favicon.svg">
<meta name="theme-color" media="(prefers-color-scheme: dark)" content="#10130f">
<meta name="theme-color" media="(prefers-color-scheme: light)" content="#f3f1e8">
<style>
:root{color-scheme:light dark;--bg:#10130f;--panel:#191e16;--ink:#e9ece3;--ink2:#d4dac9;--mut:#9aa394;--line:#2b3327;--line2:#33402c;--green:#3fae6f;--cream:#efe7d2;--soft:#222a1f;--tip:#0c0f0b;--imgbg:#0c0f0b;--btn:#efe7d2;--btn-ink:#10130f;--chipg:#7fd6a3;--chipgbg:#1d3a2a;--chipa:#e3c987;--chipabg:#3a3320;--chipr:#ef9b8f;--amber:#d8a13a}
@media(prefers-color-scheme:light){:root{--bg:#f3f1e8;--panel:#ffffff;--ink:#1b2016;--ink2:#3b4231;--mut:#5f6a52;--line:#e4dfce;--line2:#d6cfb8;--green:#2c8a55;--cream:#23301d;--soft:#efebdc;--tip:#ffffff;--imgbg:#e9e4d6;--btn:#2c8a55;--btn-ink:#ffffff;--chipg:#1d7a44;--chipgbg:#dceee0;--chipa:#86600f;--chipabg:#f3e9c9;--chipr:#b3442e;--amber:#946610}}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--ink);font:16px/1.55 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,system-ui,sans-serif;-webkit-font-smoothing:antialiased}
.wrap{max-width:1040px;margin:0 auto;padding:2.4rem 1.2rem 5rem}
header{border-bottom:1px solid var(--line);padding-bottom:1.3rem;margin-bottom:1.6rem}
header h1{font-size:1.7rem;margin:.2rem 0;letter-spacing:-.01em}
header p{color:var(--mut);margin:.25rem 0 0}
.about{background:var(--panel);border:1px solid var(--line);border-radius:16px;padding:1.1rem 1.3rem;margin-bottom:1.7rem}
.about p{margin:0;color:var(--ink2)}
.about .fine{margin:.8rem 0 0;color:var(--mut);font-size:.88rem}
.funnel{display:flex;align-items:stretch;gap:.7rem;margin:1.4rem 0 .8rem;flex-wrap:wrap}
.fstat{flex:1;min-width:96px;background:var(--panel);border:1px solid var(--line);border-radius:14px;padding:.85rem 1rem;display:flex;flex-direction:column;gap:.1rem}
.fstat.hit{border-color:var(--green);background:var(--chipgbg)}
.fnum{font-size:1.85rem;font-weight:800;letter-spacing:-.02em;color:var(--ink);line-height:1.05}
.fstat.hit .fnum{color:var(--green)}
.flbl{font-size:.76rem;color:var(--mut);line-height:1.25}
.fstat.hit .flbl{color:var(--green)}
.upd{margin:0 0 1.7rem;color:var(--mut);font-size:.82rem}
.critgroup{margin-top:1.1rem}
.crittag{display:inline-block;font-size:.72rem;text-transform:uppercase;letter-spacing:.05em;font-weight:700;padding:.25rem .6rem;border-radius:7px;margin-bottom:.6rem}
.crittag.req{background:var(--chipgbg);color:var(--chipg)}
.crittag.plus{background:var(--chipabg);color:var(--chipa)}
.chips{display:flex;flex-wrap:wrap;gap:.5rem}
.chips span{background:var(--soft);border:1px solid var(--line);color:var(--ink);font-size:.82rem;padding:.34rem .72rem;border-radius:999px}
.chips.plus span{border-style:dashed;color:var(--mut);background:transparent}
.explain{margin-bottom:1.9rem}
.exh{font-size:.78rem;text-transform:uppercase;letter-spacing:.1em;color:var(--mut);font-weight:700;margin:0 0 .9rem}
.explain details{background:var(--panel);border:1px solid var(--line);border-radius:14px;margin-bottom:.6rem;overflow:hidden;transition:border-color .15s}
.explain details[open]{border-color:var(--line2)}
.explain summary{cursor:pointer;list-style:none;padding:.95rem 1.2rem;font-weight:600;color:var(--ink);display:flex;justify-content:space-between;align-items:center;gap:1rem}
.explain summary::-webkit-details-marker{display:none}
.explain summary::after{content:'+';color:var(--green);font-size:1.35rem;font-weight:400;line-height:1}
.explain details[open] summary::after{content:'–'}
.explain summary:hover{background:var(--soft)}
.explain .body{padding:0 1.2rem 1.1rem;color:var(--ink2);line-height:1.62;font-size:.95rem}
.explain .body p{margin:.1rem 0 .7rem}.explain .body p:last-child{margin-bottom:0}
.explain .body ul{margin:.2rem 0 0;padding-left:1.1rem}
.explain .body li{margin:.3rem 0}
.explain .body b{color:var(--ink)}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(270px,1fr));gap:1.1rem;align-items:stretch}
.tile{display:flex;flex-direction:column;height:100%;background:var(--panel);border:1px solid var(--line);border-radius:16px;overflow:hidden;text-decoration:none;color:inherit;transition:border-color .15s,transform .15s,box-shadow .15s}
.tile:hover{border-color:var(--green);transform:translateY(-3px);box-shadow:0 10px 26px rgba(0,0,0,.32)}
.tile.almost:hover{border-color:var(--amber)}
.tag{position:absolute;top:.6rem;left:.6rem;z-index:2;font-size:.7rem;font-weight:700;padding:.28rem .6rem;border-radius:8px;letter-spacing:.02em;background:rgba(12,15,11,.72);-webkit-backdrop-filter:blur(4px);backdrop-filter:blur(4px)}
.tag.near{color:#e3b65f}
.tag.pin{color:#7fd6a3}
.tag.outlier{color:#e3b341}
.offspec{margin-top:.35rem;color:var(--mut);font-size:.78rem;line-height:1.4}
.merits{display:flex;flex-wrap:wrap;gap:.35rem;margin:.5rem 0 .1rem}
.bug{font-size:.72rem;font-weight:700;padding:.2rem .55rem;border-radius:999px;white-space:nowrap}
.bug.priv{background:rgba(63,174,111,.16);color:var(--chipg)}
.b-matala{background:rgba(63,174,111,.16);color:var(--chipg)}
.b-kohtalainen{background:rgba(216,161,58,.18);color:var(--chipa)}
.b-korkea{background:rgba(224,90,74,.18);color:var(--chipr)}
.sechead{margin:2.3rem 0 1rem}
.almostlede{margin:-.3rem 0 1.2rem;color:var(--mut);font-size:.92rem;line-height:1.55;max-width:64ch}
.almostlede b{color:var(--ink2)}
.thumb{position:relative;aspect-ratio:3/2;overflow:hidden;background:var(--imgbg)}
.thumb img{position:absolute;inset:0;width:100%;height:100%;object-fit:cover}
.meta{display:flex;flex-direction:column;flex:1;padding:.85rem .95rem 1rem}
.place{color:var(--mut);font-size:.9rem}
.row{display:flex;justify-content:space-between;align-items:baseline;gap:.5rem;margin:.2rem 0}
.p{font-size:1.3rem;font-weight:800;color:var(--cream);letter-spacing:-.02em}
.m{color:var(--mut);font-size:.8rem;text-align:right;white-space:nowrap}
.facts2{color:var(--mut);font-size:.82rem;margin-top:auto;padding-top:.5rem;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
footer{color:var(--mut);font-size:.85rem;text-align:center;margin-top:2.8rem;padding-top:1.4rem;border-top:1px solid var(--line)}
@media(max-width:560px){.wrap{padding:1.6rem 1rem 4rem}header h1{font-size:1.45rem}.grid{gap:.9rem}}
</style></head><body><div class="wrap">
<header><h1>Kontu — validoidut kohteet</h1><p>Suomen myynnissä olevat ranta-asunnot yhden tiukan laatuseulan läpi.</p></header>
<div class="funnel">
  <div class="fstat"><span class="fnum">${thousands(scanned)}</span><span class="flbl">ranta-ilmoitusta arvioitu</span></div>
  <div class="fstat"><span class="fnum">${munis}</span><span class="flbl">kuntaa eri puolilla Suomea</span></div>
  <div class="fstat hit"><span class="fnum">${n}</span><span class="flbl">läpäisi laatuseulan</span></div>
</div>
${updated || unread ? `<p class="upd">${updated ? `Tiedot päivitetty ${updated} · ` : ""}vain laatuseulan läpäisseet näkyvät täällä.${unread ? ` (${unread} ilmoitusta vain Etuovessa jäi arvioimatta — tietoja ei voitu lukea.)` : ""}</p>` : ""}
<section class="about">
<p>Nämä eivät ole satunnainen lista — kohteet ovat <b>läpäisseet kontun laatuseulan</b>. Algoritmi käy läpi Suomen myynnissä olevat rantakohteet ja päästää listalle vain ne, jotka täyttävät <b>kaikki pakolliset</b> kriteerit. Jokaisesta on lisäksi mallinnettu todelliset asumiskulut ja ostajan riski paikallisilla kustannusmalleilla.</p>
<div class="critgroup"><span class="crittag req">Pakolliset — kaikkien täytyttävä</span>
<div class="chips"><span>Yksitasoinen</span><span>Oma järvenranta (ei lampi, joki tai meri)</span><span>Kuntoluokka hyvä tai parempi</span><span>Matala ostajan riski (≤ 25/100)</span><span>Ympärivuotinen (talviasuttava)</span><span>Oma tontti</span><span>Toimiva infra: vesi, viemäri, tie, sähkö</span><span>≤ 100 000 €</span><span>Käteiskaupan hinta</span></div></div>
<div class="critgroup"><span class="crittag plus">Plussaa — ei pakollinen, mutta nostaa sijoitusta</span>
<div class="chips plus"><span>Valokuitu (erittäin hyvä etätyölle)</span><span>Rauhallinen · ei lähinaapureita</span><span>Sähköauton lataus</span><span>Matala kokonaiskustannus</span></div></div>
</section>
<section class="explain">
<h2 class="exh">Miten algoritmi toimii</h2>
<details open><summary>Mikä tämä lista on?</summary><div class="body">
<p>Tämä ei ole hakukone vaan <b>käsin määritellyn algoritmin — kontun laatuseulan — läpäisseiden kotien lista</b>. Algoritmi käy läpi Suomen myynnissä olevat rantakohteet ja päästää listalle vain ne, jotka täyttävät jokaisen pakollisen kriteerin. Jokaisesta on lisäksi laskettu todelliset asumiskulut ja ostajan riski.</p></div></details>
<details><summary>Miten laatuseula toimii?</summary><div class="body">
<p>Seula on <b>binäärinen</b>: kohde joko läpäisee <i>kaikki</i> pakolliset kriteerit tai ei ole listalla. Ei pisteytystä, ei kompromisseja — yksikin täyttymätön pakollinen kriteeri pudottaa kohteen. Plussat eivät vaikuta läpäisyyn, vaan järjestävät läpäisseet keskenään.</p>
<p>Vaiheet: <b>1)</b> haetaan markkina (Oikotie · Etuovi), <b>2)</b> rikastetaan jokainen kohde detaljisivun tiedoilla, <b>3)</b> lasketaan kustannukset ja ostajan riski paikallisesti, <b>4)</b> sovelletaan seula.</p></div></details>
<details><summary>Miksi juuri nämä kriteerit?</summary><div class="body"><ul>
<li><b>Yksitasoinen</b> — ei portaita, esteetön ja helppo ylläpitää.</li>
<li><b>Oma järvenranta</b> — järvi, ei lampi, joki eikä meri; oma rantaviiva, ei pelkkä rantaoikeus.</li>
<li><b>Kuntoluokka hyvä tai parempi</b> — muuttovalmis, ei remonttiprojekti.</li>
<li><b>Matala ostajan riski (≤ 25)</b> — vähän lykättyä korjausvelkaa (putket, salaojat, ikärakenteet).</li>
<li><b>Ympärivuotinen</b> — talviasuttava koti, ei kesämökki.</li>
<li><b>Oma tontti + toimiva infra</b> — ei vuokratonttia; vesi, viemäri, tie perille, sähkö.</li>
<li><b>≤ 100 000 € · käteinen</b> — ostetaan ilman lainaa, ilman vastiketta.</li>
</ul></div></details>
<details><summary>Asumiskulut — miten lasketaan?</summary><div class="body">
<p><b>Asumiskulut on kuukausittainen ylläpito yhteensä</b>: lämmitys, sähkö, vakuutus, kiinteistövero ja ylläpito. Ei sisällä lainanlyhennystä eikä vastiketta (käteiskauppa, oma tontti). Lisäksi mallinnetaan <b>20 vuoden kokonaiskustannus</b> (nettonykyarvo), joka sisältää lykätyn korjausvelan. Mallit ovat paikallisia ja deterministisiä — samat luvut joka kerta.</p></div></details>
<details><summary>Ostajan riski — miten lasketaan?</summary><div class="body">
<p><b>0–100 pisteen malli</b> arvioi lykätyn korjausvelan ja ikäriskin: rakennusvuosi ja -aika (esim. 1960–85 riskirakenteet), putkiremontin ja salaojien ajankohta, katon ikä, lämmitysmuoto (öljy) ja kuntotarkastuksen olemassaolo. Pienempi on parempi; seula vaatii <b>≤ 25</b>.</p></div></details>
<details><summary>Mistä tiedot tulevat?</summary><div class="body">
<p><b>Ilmoitukset:</b> Oikotie ja Etuovi (haetaan kotikoneelta, koska portaalit estävät datakeskusten IP:t). <b>Sijainti &amp; ympäristö:</b> avoin valtion paikkatieto (SYKE, Maanmittauslaitos, OSM). <b>Kustannus- ja riskimallit</b> lasketaan paikallisesti. Kuvat haetaan suoraan alkuperäisen ilmoituksen palvelimelta.</p></div></details>
<details><summary>Entä mäkärät ja hyttyset?</summary><div class="body">
<p>Lasketaan <b>pehmeänä lisätietona</b> — ei pakollisena kriteerinä, ei karsi kohteita. Jokaiselle kohteelle arvioidaan kaksi erillistä indeksiä koordinaateista avoimesta paikkatiedosta:</p>
<ul>
<li><b>Hyttyset</b> seisovasta vedestä: <b>MML maastotietokannan suo-aineisto</b> (todelliset suo- ja kosteikkopolygonit 2 km alueelta) sekä järviala.</li>
<li><b>Mäkärät</b> virtaavasta vedestä: <b>SYKE:n uomaverkosto</b> (purojen ja jokien määrä 2,5 km säteellä).</li>
</ul>
<p>Lisäksi lievä pohjoisuuskorjaus (räkkä pahenee pohjoiseen). Tulos näkyy kohdesivulla matala/kohtalainen/korkea -bändinä. Huom: <b>järvivaatimus jo vähentää mäkäriä</b> — ne lisääntyvät vain virtaavassa vedessä, ei järvenrannalla. Kausi painottuu kesä–heinäkuuhun ja vaihtelee sään mukaan, joten tämä on suuntaa-antava arvio, ei mittaus.</p></div></details>
</section>
<h2 class="exh sechead">Validoidut · ${n}</h2>
<div class="grid">${gateCards || '<p class="m">Ei juuri nyt yhtään laatuseulan läpäissyttä kohdetta.</p>'}</div>
${
  almostN
    ? `<h2 class="exh sechead">Melkein läpäisi · ${almostN}</h2>
<p class="almostlede">Nämä täyttävät <b>kaikki pakolliset kriteerit</b> ja kunto on vahvistettu hyväksi — vain mallinnettu <b>ostajan riski</b> ylittää seulan tiukan ≤25 rajan. Eivät siis validoituja, mutta varteenotettavia.</p>
<div class="grid">${almostCards}</div>`
    : ""
}
${
  outlierN
    ? `<h2 class="exh sechead">Aliarvostetut löydöt · ${outlierN}</h2>
<p class="almostlede">Nämä <b>eivät täytä kaikkia toiveita</b> (yleensä ei omaa järvenrantaa) — mutta jos rannasta tinkii, vastineeksi vaaditaan muuta: hinta on <b>selvästi alle alueen mediaanin</b>, sijainti on <b>rauhallinen</b> ja <b>hyttys-/mäkäräpaine vähäinen</b> (vain matalan räkän kohteet pääsevät tänne — kohtalainenkin paine karsii). Kunkin kortin kärjessä se mitä saa, alla mistä tinkii.</p>
<div class="grid">${outlierCards}</div>`
    : ""
}
<footer>Koottu kontulla · luvut virallisista ilmoituksista ja paikallisista kustannusmalleista</footer>
</div></body></html>`;
}

export function renderListingPage(p: PublishedPayload, origin: string): string {
  const cover = p.gallery[0] ?? "";
  const monthly = Math.round(p.cost.monthly_living);
  const cash = p.cost.cash;
  const title = p.address || p.title || p.municipality || `Kohde #${p.id}`;
  const sub = [p.municipality, p.property_type].filter(Boolean).join(" · ");

  const gallery = p.gallery
    .map(
      (u, i) =>
        `<img src="${esc(u)}" loading="${i === 0 ? "eager" : "lazy"}" alt="Kuva ${i + 1}" referrerpolicy="no-referrer">`,
    )
    .join("");

  const T = {
    ppm2: "Velaton hinta jaettuna asuinpinta-alalla — vertailuluku eri kohteiden välillä.",
    kunto: "Asunnon yleiskunto: hyvä = muuttovalmis, tyydyttävä = pientä päivitystä, välttävä/huono = remontoitava.",
    energia: "Energiatehokkuusluokka A–G (A paras). Vaikuttaa lämmityskuluihin.",
    remontit: "Merkittävät tehdyt remontit ja vuosi — katto ja putket ovat kalleimmat.",
    lammitys: "Päälämmitysmuoto. Vaikuttaa käyttökuluihin ja päästöihin.",
    vesi: "Talousveden lähde. Kunnallinen on huolettomin; kaivo vaatii huoltoa.",
    jatevesi: "Jätevesien käsittely. Saostuskaivo tai umpisäiliö voi vaatia päivityksen (haja-asutuksen jätevesiasetus 157/2017).",
    netti: "Käytettävissä oleva nettiyhteys. Valokuitu on nopein ja vakain — tärkeä etätyölle.",
    ev: "Sähköauton latausmahdollisuus tai -valmius kohteessa.",
    sauna: "Onko kohteessa sauna — rantasauna on erillinen rakennus rannassa.",
    ranta: "Rannan omistusmuoto (oma ranta / rantaoikeus) ja vesistön tyyppi.",
    rantaviiva: "Oman rantaviivan pituus metreinä.",
    naapurit: "Arvio naapureiden läheisyydestä ilmoituksen ja tontin koon perusteella.",
    tie: "Pääseekö perille autolla ympäri vuoden. Yksityistiellä voi olla tiemaksu.",
    palvelut: "Arvioitu etäisyys lähimpiin palveluihin (kauppa, keskusta).",
    tonttiOm: "Oma tontti vs. vuokratontti — vuokratontista maksetaan jatkuvaa vuokraa.",
    asumiskulut: "Kuukausittainen ylläpito YHTEENSÄ — sisältää lämmityksen, sähkön, vakuutuksen, kiinteistöveron ja ylläpidon. Ei lainanlyhennystä, ei vastiketta.",
    kvero: "Kunnan perimä vuotuinen kiinteistövero.",
    sahko: "Arvioitu vuotuinen sähkönkulutuksen kustannus.",
    kokonais: `Mallinnettu omistamisen nettonykyarvo ${p.cost.horizon_years} vuodelle — sisältää lykätyn korjausvelan.`,
    riski: "0–100 mallinnettu ostajan riski (ikä, riskirakenteet, lykätty korjausvelka). Pienempi on parempi.",
  };

  const propFacts = [
    fact("Asuinpinta-ala", p.living_area_m2 != null ? `${thousands(p.living_area_m2)} m²` : "—"),
    fact("Hinta / m²", ppm2(p) ?? "—", T.ppm2),
    fact("Tontti", area(p.plot_area_m2)),
    fact("Rakennusvuosi", p.year_built != null ? String(p.year_built) : "—"),
    fact("Rakennusmateriaali", materiaaliFi(p) ?? "—"),
    fact("Huoneet", p.room_count != null ? String(p.room_count) : "—"),
    fact("Kuntoluokka", p.condition_class ?? "—", T.kunto),
    fact("Energialuokka", p.energy_class ?? "—", T.energia),
    fact("Tehdyt remontit", remontitFi(p) ?? "—", T.remontit),
  ].join("");

  const infraFacts = [
    fact("Lämmitys", p.heating_type ?? "—", T.lammitys),
    fact("Vesi", vesiFi(p) ?? "—", T.vesi),
    fact("Jätevesi", jatevesiFi(p) ?? "—", T.jatevesi),
    fact("Nettiyhteys", netFi(p) ?? "—", T.netti),
    fact("Auton lataus", evFi(p) ?? "—", T.ev),
    fact("Sauna", saunaFi(p) ?? "—", T.sauna),
  ].join("");

  const locFacts = [
    fact("Ranta", p.shore === "oma_ranta" ? `oma ranta${p.water_body ? ` · ${p.water_body}` : ""}` : (p.shore ?? "—"), T.ranta),
    fact("Rantaviivaa", rantaviivaFi(p) ?? "—", T.rantaviiva),
    fact("Naapurit", naapuritFi(p) ?? "—", T.naapurit),
    fact("Tieyhteys", tieFi(p) ?? "—", T.tie),
    fact("Palvelut", palvelutFi(p) ?? "—", T.palvelut),
    fact("Tontin omistus", p.plot_ownership ?? "—", T.tonttiOm),
  ].join("");

  const annualBits = [
    p.cost.kiinteistovero_eur_yr != null ? `kiinteistövero ${thousands(p.cost.kiinteistovero_eur_yr)} €/v` : "",
    p.cost.electricity_eur_yr != null ? `sähkö ${thousands(p.cost.electricity_eur_yr)} €/v` : "",
  ]
    .filter(Boolean)
    .join(" · ");
  const priceRows = [
    fact("Kauppahinta", eur(p.price_eur)),
    fact(`Kokonaiskustannus (${p.cost.horizon_years} v)`, eur(p.cost.npv_cost), T.kokonais),
  ].join("");
  const costSection = `<section class="card"><h2>Kulut</h2>
    <div class="costhero">
      <div class="costbig">≈ ${thousands(monthly)} €<span>/kk</span></div>
      <div class="costlbl"><b>Asumiskulut yhteensä</b>${info(T.asumiskulut)}<br><span class="costfine">lämmitys, sähkö, vakuutus, kiinteistövero ja ylläpito${cash ? " — ei lainaa eikä vastiketta" : ""}</span></div>
    </div>
    ${annualBits ? `<div class="costnote">Sisältää mm. ${esc(annualBits)}</div>` : ""}
    <div class="grid costgrid">${priceRows}</div>
  </section>`;

  const dataSection = (heading: string, body: string): string =>
    body ? `<section class="card"><h2>${esc(heading)}</h2><div class="grid">${body}</div></section>` : "";

  const reasons = p.reasons.length
    ? `<section class="card reasons"><h2>Miksi tämä kohde</h2><ul>${p.reasons
        .map((r) => `<li>${esc(r)}</li>`)
        .join("")}</ul></section>`
    : "";

  const riskFlags = p.risk.flags.length
    ? `<ul class="flags">${p.risk.flags
        .map(
          (f) =>
            `<li><span class="pts">+${f.points}</span><span class="flabel">${esc(riskFi(f.label))}${
              f.capex_eur > 0 ? ` <span class="capex">~${thousands(f.capex_eur / 1000)} k€</span>` : ""
            }</span></li>`,
        )
        .join("")}</ul>`
    : `<p class="muted">Ei merkittäviä riskimerkintöjä.</p>`;

  const map =
    p.lat != null && p.lon != null
      ? `<section class="card"><h2>Kartta &amp; lähistö ${info("Interaktiivinen kartta (MapLibre + OpenStreetMap). Vihreä merkki on kohde; muut merkit ovat lähimmät palvelut, jotka haetaan OpenStreetMapista. Maaseutukohteessa palveluja on luonnollisesti vähän.")}</h2>
         <div id="map" class="mlmap" data-lat="${p.lat}" data-lon="${p.lon}"></div>
         <div id="nearby" class="nearby"><span class="muted">Haetaan lähipalveluja…</span></div>
         <a class="maplink" href="https://www.openstreetmap.org/?mlat=${p.lat}&mlon=${p.lon}#map=13/${p.lat}/${p.lon}" target="_blank" rel="noopener">Avaa isompi kartta →</a></section>`
      : "";

  const bugs = renderBugPressure(p.bug_pressure);

  const description = p.description
    ? `<section class="card"><h2>Kuvaus</h2><p class="desc">${esc(p.description)}</p></section>`
    : "";

  const cashLine = cash
    ? `<p class="cashnote">Käteiskauppa — ei asuntolainaa, ei velkaa, ei pankkia. Ei toistuvia maksuja: ei vastiketta, ei tonttivuokraa.</p>`
    : "";

  const offSpecLine = p.off_spec?.length ? esc(p.off_spec.join(", ")) : "ei kaikkia toiveita";
  const bugSummary = p.bug_pressure
    ? ` Hyttyspaine ${esc(p.bug_pressure.mosquito.band)}, mäkäräpaine ${esc(p.bug_pressure.blackfly.band)}.`
    : "";
  const tierBanner =
    p.tier === "gate"
      ? ""
      : p.tier === "outlier"
        ? `<div class="tierbanner outlier">
          <span class="tbt">${hasLakeShore(p) ? "Aliarvostettu löytö — oma ranta" : "Aliarvostettu löytö — rauhallinen, vähän bugeja"}</span>
          <span class="tbd">${
            hasLakeShore(p)
              ? `Tässä on <b>oma järvenranta</b> ja hinta on <b>selvästi alle alueen mediaanin</b> — mutta se jää muista toiveista: <b>${offSpecLine}</b>.${bugSummary} Arvioi itse, korvaako hinta ja ranta puuttuvan toiveen.`
              : `Hinta on <b>selvästi alle alueen mediaanin</b>, sijainti on <b>rauhallinen</b> ja <b>hyttysiä/mäkäriä on vähän</b>.${bugSummary} Tinkimisen paikka: <b>${offSpecLine}</b>. Arvioi itse, korvaako hinta ja rauha puuttuvan järvenrannan.`
          }</span>
        </div>`
        : `<div class="tierbanner ${p.tier === "pin" ? "pin" : "near"}">
          <span class="tbt">${p.tier === "pin" ? "★ Suosikki — melkein läpäisi" : "Melkein läpäisi seulan"}</span>
          <span class="tbd">Täyttää kaikki pakolliset kriteerit ja kunto on vahvistettu hyväksi — vain mallinnettu ostajan riski <b>${p.risk.score}/100</b> ylittää seulan tiukan ≤25 rajan. Ei siis "validoitu", mutta varteenotettava.</span>
        </div>`;

  return `<!doctype html>
<html lang="fi"><head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex, nofollow">
<title>${esc(title)} — ${eur(p.price_eur)} | kontu</title>
<meta property="og:type" content="website">
<meta property="og:title" content="${esc(title)} — ${eur(p.price_eur)}">
<meta property="og:description" content="${esc(sub)} · ~${thousands(monthly)} €/kk · ${esc(p.condition_class ?? "")}">
${cover ? `<meta property="og:image" content="${esc(cover)}">` : ""}
<meta name="twitter:card" content="summary_large_image">
<meta name="theme-color" media="(prefers-color-scheme: dark)" content="#10130f">
<meta name="theme-color" media="(prefers-color-scheme: light)" content="#f3f1e8">
<link rel="icon" type="image/svg+xml" href="/favicon.svg">
<link href="https://unpkg.com/maplibre-gl@5.6.0/dist/maplibre-gl.css" rel="stylesheet">
<style>
:root{color-scheme:light dark;--bg:#10130f;--panel:#191e16;--ink:#e9ece3;--ink2:#d4dac9;--mut:#9aa394;--line:#2b3327;--line2:#33402c;--green:#3fae6f;--cream:#efe7d2;--soft:#222a1f;--tip:#0c0f0b;--imgbg:#0c0f0b;--btn:#efe7d2;--btn-ink:#10130f;--chipg:#7fd6a3;--chipgbg:#1d3a2a;--chipa:#e3c987;--chipabg:#3a3320;--chipr:#ef9b8f;--amber:#d8a13a}
@media(prefers-color-scheme:light){:root{--bg:#f3f1e8;--panel:#ffffff;--ink:#1b2016;--ink2:#3b4231;--mut:#5f6a52;--line:#e4dfce;--line2:#d6cfb8;--green:#2c8a55;--cream:#23301d;--soft:#efebdc;--tip:#ffffff;--imgbg:#e9e4d6;--btn:#2c8a55;--btn-ink:#ffffff;--chipg:#1d7a44;--chipgbg:#dceee0;--chipa:#86600f;--chipabg:#f3e9c9;--chipr:#b3442e;--amber:#946610}}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--ink);font:16px/1.6 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,system-ui,sans-serif;-webkit-font-smoothing:antialiased;text-rendering:optimizeLegibility}
a{color:var(--green)}
.wrap{max-width:820px;margin:0 auto;padding:0 0 5rem}
.herowrap{position:relative}
.dtop{position:absolute;top:0;left:0;right:0;z-index:3;display:flex;justify-content:space-between;padding:12px 14px;background:linear-gradient(rgba(0,0,0,.45),transparent);pointer-events:none}
.dbtn{pointer-events:auto;width:40px;height:40px;display:flex;align-items:center;justify-content:center;background:rgba(16,19,15,.66);color:#fff;border:1px solid var(--line);border-radius:50%;text-decoration:none;font-size:19px;cursor:pointer;-webkit-backdrop-filter:blur(5px);backdrop-filter:blur(5px)}
.dbtn:active{transform:scale(.94)}
.photocount{position:absolute;right:1.4rem;bottom:1.3rem;z-index:3;background:rgba(16,19,15,.74);color:#fff;font-size:.78rem;padding:5px 11px;border-radius:999px;border:1px solid var(--line);cursor:pointer;-webkit-backdrop-filter:blur(4px);backdrop-filter:blur(4px)}
.hero{display:flex;gap:12px;overflow-x:auto;scroll-snap-type:x mandatory;padding:.85rem 16px;scroll-padding-inline:16px;scrollbar-width:none;-webkit-overflow-scrolling:touch}
.hero::-webkit-scrollbar{display:none}
.hero img{flex:0 0 84%;min-width:0;width:84%;aspect-ratio:4/3;object-fit:contain;border-radius:14px;scroll-snap-align:center;scroll-snap-stop:always;background:var(--imgbg);cursor:zoom-in}
.head{padding:1.4rem 1.5rem .4rem}
.head .sub{color:var(--mut);font-size:.95rem}
.head h1{margin:.15rem 0;font-size:1.75rem;line-height:1.18;letter-spacing:-.01em}
.price{font-size:2.1rem;font-weight:800;color:var(--cream);margin:.55rem 0 .2rem;letter-spacing:-.02em}
.cashnote{color:var(--green);font-weight:600;font-size:.95rem;margin:.3rem 0 0}
.tierbanner{margin:.9rem 1.5rem 0;padding:.85rem 1.05rem;border-radius:12px;background:var(--soft);border-left:3px solid var(--amber)}
.tierbanner.pin{border-left-color:var(--green)}
.tbt{display:block;font-weight:800;font-size:.92rem;color:var(--amber);margin-bottom:.2rem}
.tierbanner.pin .tbt{color:var(--green)}
.tbd{color:var(--ink2);font-size:.87rem;line-height:1.5}
section{margin:1.1rem 1.5rem 0}
.card{background:var(--panel);border:1px solid var(--line);border-radius:18px;padding:1.25rem 1.4rem}
h2{display:flex;justify-content:space-between;align-items:baseline;gap:.6rem;font-size:.78rem;text-transform:uppercase;letter-spacing:.1em;color:var(--mut);margin:.1rem 0 1rem;font-weight:700}
.riskscore{font-weight:800;color:var(--cream);letter-spacing:0;text-transform:none;font-size:.95rem;white-space:nowrap}
.grid{display:grid;grid-template-columns:1fr 1fr;gap:0 1.6rem}
.fact{display:flex;justify-content:space-between;gap:1rem;padding:.6rem 0;border-bottom:1px solid var(--line)}
.fact dt{color:var(--mut);margin:0}.fact dd{margin:0;text-align:right;font-weight:600}
.costhero{display:flex;align-items:center;gap:1rem;flex-wrap:wrap}
.costbig{font-size:2.1rem;font-weight:800;color:var(--cream);letter-spacing:-.02em;white-space:nowrap;line-height:1}
.costbig span{font-size:1rem;font-weight:600;color:var(--mut)}
.costlbl{color:var(--ink);font-size:.95rem;line-height:1.35;flex:1;min-width:11rem}
.costfine{color:var(--mut);font-size:.82rem}
.costnote{color:var(--mut);font-size:.85rem;border-top:1px solid var(--line);padding-top:.7rem;margin-top:.9rem}
.costgrid{margin-top:.85rem;border-top:1px solid var(--line);padding-top:.1rem}
.info{display:inline-flex;align-items:center;justify-content:center;width:15px;height:15px;margin-left:4px;border:1px solid var(--line);background:var(--soft);color:var(--mut);border-radius:50%;font:italic 700 10px/1 Georgia,serif;cursor:pointer;vertical-align:middle;padding:0}
.info:hover{color:var(--ink);border-color:var(--green)}
#tip{position:fixed;z-index:60;max-width:260px;background:var(--tip);border:1px solid var(--green);color:var(--ink);font-size:.82rem;line-height:1.45;padding:.6rem .75rem;border-radius:10px;box-shadow:0 8px 24px rgba(0,0,0,.5);display:none}
#tip.show{display:block}
.lb[hidden]{display:none}
.lb{position:fixed;inset:0;z-index:70;background:#060805}
.lb-stage{position:absolute;inset:0;display:flex;align-items:center;justify-content:center;overflow:hidden;touch-action:none}
.lb-img{max-width:100vw;max-height:100vh;object-fit:contain;transform-origin:center center;will-change:transform;user-select:none;-webkit-user-drag:none;touch-action:none}
.lb-top{position:absolute;top:0;left:0;right:0;z-index:4;display:flex;align-items:center;justify-content:space-between;gap:12px;padding:12px 14px;background:linear-gradient(rgba(0,0,0,.55),transparent)}
.lb-count{color:#fff;font-size:.85rem;font-variant-numeric:tabular-nums}
.lb button{background:rgba(25,30,22,.82);color:#fff;border:1px solid var(--line);border-radius:999px;cursor:pointer;display:flex;align-items:center;justify-content:center;line-height:1}
.lb-top button{width:40px;height:40px;font-size:18px}
.lb-nav{position:absolute;top:50%;transform:translateY(-50%);z-index:4;width:46px;height:46px;font-size:28px}
.lb-prev{left:12px}.lb-next{right:12px}
.lb-thumbs{position:absolute;inset:0;z-index:3;display:none;grid-template-columns:repeat(2,1fr);grid-auto-rows:min-content;gap:8px;overflow-y:auto;padding:64px 10px 20px;background:#060805;-webkit-overflow-scrolling:touch}
.lb-thumbs img{width:100%;aspect-ratio:4/3;object-fit:cover;border-radius:8px;cursor:pointer;background:#0c0f0b}
.lb.grid .lb-thumbs{display:grid}
.lb.grid .lb-stage,.lb.grid .lb-nav{display:none}
@media(min-width:561px){.lb-img{max-width:94vw;max-height:92vh}}
.reasons ul{margin:0;padding-left:1.15rem}.reasons li{margin:.3rem 0;color:var(--ink2)}
.flags{list-style:none;margin:0;padding:0}
.flags li{display:flex;gap:.7rem;align-items:baseline;padding:.55rem 0;border-bottom:1px solid var(--line)}
.flags li:last-child{border-bottom:0}
.pts{flex:0 0 auto;min-width:2.1rem;text-align:center;color:var(--mut);font-variant-numeric:tabular-nums;font-size:.82rem;background:var(--soft);border-radius:6px;padding:1px 0}
.flabel{flex:1}
.capex{color:var(--amber);font-size:.85em;white-space:nowrap}
.desc{white-space:pre-wrap;color:var(--ink2);margin:0}
.mlmap{width:100%;height:360px;border-radius:14px;overflow:hidden;background:var(--imgbg)}
.mlmap .maplibregl-ctrl-attrib{font-size:10px}
.mlmap .maplibregl-popup-content{background:#171c12;color:#efe7d2;border-radius:10px;padding:.5rem .7rem;font-size:.85rem;box-shadow:0 6px 24px rgba(0,0,0,.5)}
.mlmap .maplibregl-popup-tip{border-top-color:#171c12;border-bottom-color:#171c12}
.mlmap .maplibregl-popup-close-button{color:var(--mut)}
.poi{width:12px;height:12px;border-radius:50%;background:#efe7d2;border:2px solid #0c0f0a;box-shadow:0 0 0 1px rgba(239,231,210,.45);cursor:pointer}
.poi.poi-town{background:#d8a13a}
.nearby{margin-top:.9rem;font-size:.92rem}
.nearby .nbhead{color:var(--mut);font-size:.78rem;text-transform:uppercase;letter-spacing:.04em;margin-bottom:.35rem}
.nearby .nb{display:flex;justify-content:space-between;gap:1rem;padding:.45rem 0;border-bottom:1px solid var(--line)}
.nearby .nb:last-child{border-bottom:0}
.nearby .nbk{color:var(--ink2)}.nearby .nbd{color:var(--mut);font-variant-numeric:tabular-nums;white-space:nowrap}
.maplink{display:inline-block;margin-top:.9rem;text-decoration:none}
.bugrow{display:flex;justify-content:space-between;align-items:center;gap:1rem;padding:.6rem 0;border-bottom:1px solid var(--line)}
.bugrow:last-of-type{border-bottom:0}
.buglabel{color:var(--ink2)}
.bugband{font-weight:700;font-size:.8rem;padding:.22rem .7rem;border-radius:999px;text-transform:capitalize;white-space:nowrap}
.b-matala{background:rgba(63,174,111,.16);color:var(--chipg)}
.b-kohtalainen{background:rgba(216,161,58,.18);color:var(--chipa)}
.b-korkea{background:rgba(224,90,74,.18);color:var(--chipr)}
.bugbasis{color:var(--mut);font-size:.82rem;margin:.8rem 0 0;line-height:1.55}
.muted{color:var(--mut)}
.src{display:block;margin:1.4rem 1.5rem 0}
.src a{display:block;text-align:center;background:var(--btn);color:var(--btn-ink);font-weight:700;padding:1rem;border-radius:14px;text-decoration:none;transition:transform .12s,filter .12s}
.src a:hover{filter:brightness(1.05);transform:translateY(-1px)}
.foot{margin:2rem 1.5rem 0;color:var(--mut);font-size:.85rem;text-align:center;line-height:1.5}
@media(max-width:560px){
  .wrap{padding-bottom:4rem}
  .grid{grid-template-columns:1fr;gap:0}
  section{margin:1rem 1rem 0}
  .head{padding:1.4rem 1rem .4rem}
  .head h1{font-size:1.5rem}
  .price{font-size:1.85rem}
  .hero{padding:.6rem 12px}
  .hero img{flex:0 0 86%;width:86%}
  .lb-nav{width:40px;height:40px;font-size:24px}
  .src{margin:1.2rem 1rem 0}.foot{margin:1.6rem 1rem 0}
}
</style></head>
<body><div class="wrap">
<div class="herowrap">
<div class="dtop">
<a class="dbtn" href="${origin}/kontu" aria-label="Takaisin listaan">←</a>
<button class="dbtn" id="share" type="button" aria-label="Jaa">⤴</button>
</div>
<div class="hero">${gallery || '<div style="color:#555;margin:auto">ei kuvia</div>'}</div>
${p.gallery.length > 1 ? `<button class="photocount" id="pcount" type="button">▦ ${p.gallery.length} kuvaa</button>` : ""}
</div>
<div class="head">
  <div class="sub">${esc(sub)}</div>
  <h1>${esc(title)}</h1>
  <div class="price">${eur(p.price_eur)}</div>
  ${cashLine}
</div>
${tierBanner}
${costSection}
${reasons}
${dataSection("Kohteen tiedot", propFacts)}
${dataSection("Talotekniikka & infra", infraFacts)}
${dataSection("Sijainti & ympäristö", locFacts)}
<section class="card"><h2><span>Ostajan riski${info(T.riski)}</span> <span class="riskscore">${p.risk.score}/100 · ${esc(
    bandFi(p.risk.band),
  )}</span></h2>${riskFlags}${
    p.risk.deferred_capex_eur > 0
      ? `<p class="muted" style="margin:.8rem 0 0">Arvioitu lykätty korjausvelka ~${thousands(
          p.risk.deferred_capex_eur,
        )} €</p>`
      : ""
  }</section>
${description}
${map}
${bugs}
<div class="src"><a href="${esc(p.source_url)}" target="_blank" rel="noopener">Avaa alkuperäinen ilmoitus →</a></div>
<div class="foot"><a href="${origin}/kontu">← Kaikki validoidut kohteet</a><br><br>Koottu kontulla — luvut virallisesta ilmoituksesta ja paikallisista kustannusmalleista. Kuvat: alkuperäinen ilmoitus.</div>
</div>
<div id="tip"></div>
<div id="lb" class="lb" hidden>
  <div class="lb-top">
    <button class="lb-grid" type="button" aria-label="Ruudukko">▦</button>
    <div class="lb-count"></div>
    <button class="lb-x" type="button" aria-label="Sulje">×</button>
  </div>
  <button class="lb-nav lb-prev" type="button" aria-label="Edellinen">‹</button>
  <div class="lb-stage"><img class="lb-img" alt=""></div>
  <button class="lb-nav lb-next" type="button" aria-label="Seuraava">›</button>
  <div class="lb-thumbs"></div>
</div>
<script>
(function(){
  var imgs=${JSON.stringify(p.gallery).replace(/</g, "\\u003c")};
  var lb=document.getElementById('lb');
  if(lb&&imgs.length){
    var stage=lb.querySelector('.lb-stage'),img=lb.querySelector('.lb-img'),cnt=lb.querySelector('.lb-count'),thumbs=lb.querySelector('.lb-thumbs');
    var idx=0,scale=1,tx=0,ty=0;
    function applyT(){img.style.transform='translate('+tx+'px,'+ty+'px) scale('+scale+')';}
    function reset(){scale=1;tx=0;ty=0;}
    function show(n){idx=(n+imgs.length)%imgs.length;img.src=imgs[idx];cnt.textContent=(idx+1)+' / '+imgs.length;reset();img.style.transition='';applyT();}
    function open(n){lb.classList.remove('grid');show(n);lb.hidden=false;document.body.style.overflow='hidden';}
    function close(){lb.hidden=true;document.body.style.overflow='';}
    function buildThumbs(){if(thumbs.childElementCount)return;imgs.forEach(function(u,i){var t=document.createElement('img');t.src=u;t.loading='lazy';t.referrerPolicy='no-referrer';t.addEventListener('click',function(){lb.classList.remove('grid');show(i);});thumbs.appendChild(t);});}
    Array.prototype.forEach.call(document.querySelectorAll('.hero img'),function(el,i){el.addEventListener('click',function(){open(i);});});
    var pc=document.getElementById('pcount');if(pc)pc.addEventListener('click',function(){open(0);});
    lb.querySelector('.lb-next').addEventListener('click',function(e){e.stopPropagation();show(idx+1);});
    lb.querySelector('.lb-prev').addEventListener('click',function(e){e.stopPropagation();show(idx-1);});
    lb.querySelector('.lb-x').addEventListener('click',close);
    lb.querySelector('.lb-grid').addEventListener('click',function(){buildThumbs();lb.classList.toggle('grid');});
    stage.addEventListener('click',function(e){if(e.target===stage&&scale<=1.02)close();});
    document.addEventListener('keydown',function(e){if(lb.hidden)return;if(e.key==='Escape'){if(lb.classList.contains('grid'))lb.classList.remove('grid');else close();}else if(e.key==='ArrowRight')show(idx+1);else if(e.key==='ArrowLeft')show(idx-1);});
    function dist(t){return Math.hypot(t[0].clientX-t[1].clientX,t[0].clientY-t[1].clientY);}
    var sd=0,ss=1,px=0,py=0,swipe=null,lastTap=0;
    stage.addEventListener('touchstart',function(e){
      if(e.touches.length===2){sd=dist(e.touches);ss=scale;swipe=null;}
      else if(e.touches.length===1){
        var now=Date.now();
        if(now-lastTap<300){img.style.transition='transform .15s';if(scale>1.02){reset();}else{scale=2.5;}applyT();lastTap=0;e.preventDefault();}
        else{lastTap=now;}
        if(scale>1.02){px=e.touches[0].clientX-tx;py=e.touches[0].clientY-ty;swipe=null;}else{swipe=e.touches[0].clientX;}
      }
    },{passive:false});
    stage.addEventListener('touchmove',function(e){
      img.style.transition='';
      if(e.touches.length===2){e.preventDefault();scale=Math.min(Math.max(ss*dist(e.touches)/sd,1),5);applyT();}
      else if(e.touches.length===1&&scale>1.02){e.preventDefault();tx=e.touches[0].clientX-px;ty=e.touches[0].clientY-py;applyT();}
    },{passive:false});
    stage.addEventListener('touchend',function(e){
      if(scale<1.05){img.style.transition='transform .15s';reset();applyT();}
      if(swipe!==null&&e.changedTouches.length){var dx=e.changedTouches[0].clientX-swipe;if(scale<=1.02&&Math.abs(dx)>45)show(idx+(dx<0?1:-1));swipe=null;}
    });
  }
  var sh=document.getElementById('share');
  if(sh)sh.addEventListener('click',function(){
    var url=location.href.split('?')[0];
    if(navigator.share){navigator.share({title:document.title,url:url}).catch(function(){});}
    else if(navigator.clipboard){navigator.clipboard.writeText(url);var o=sh.textContent;sh.textContent='✓';setTimeout(function(){sh.textContent=o;},1200);}
  });
  var tip=document.getElementById('tip'),cur=null;
  function hideTip(){tip.className='';cur=null;}
  document.addEventListener('click',function(e){
    var b=e.target.closest?e.target.closest('.info'):null;
    if(b){
      e.preventDefault();e.stopPropagation();
      if(cur===b){hideTip();return;}
      tip.textContent=b.getAttribute('data-tip')||'';tip.className='show';cur=b;
      var r=b.getBoundingClientRect(),tw=Math.min(260,window.innerWidth-16);
      tip.style.maxWidth=tw+'px';
      tip.style.left=Math.min(Math.max(8,r.left-tw/2+8),window.innerWidth-tw-8)+'px';
      tip.style.top=(r.bottom+8)+'px';
      var th=tip.getBoundingClientRect().height;
      if(r.bottom+8+th>window.innerHeight)tip.style.top=Math.max(8,r.top-th-8)+'px';
      return;
    }
    if(cur&&e.target!==tip)hideTip();
  });
  window.addEventListener('scroll',hideTip,{passive:true});
})();
</script>
<script src="https://unpkg.com/maplibre-gl@5.6.0/dist/maplibre-gl.js"></script>
<script>
(function(){
  var el=document.getElementById('map');
  if(!el||!window.maplibregl)return;
  var LAT=parseFloat(el.dataset.lat),LON=parseFloat(el.dataset.lon);
  if(!isFinite(LAT)||!isFinite(LON))return;
  var map=new maplibregl.Map({container:'map',style:'https://tiles.openfreemap.org/styles/liberty',center:[LON,LAT],zoom:11.3,attributionControl:{compact:true}});
  map.addControl(new maplibregl.NavigationControl({showCompass:false}),'top-right');
  map.scrollZoom.disable();
  new maplibregl.Marker({color:'#3fae6f'}).setLngLat([LON,LAT]).setPopup(new maplibregl.Popup({offset:26}).setText('Kohde')).addTo(map);
  var LABELS={supermarket:'Ruokakauppa',convenience:'Lähikauppa',general:'Kauppa',department_store:'Tavaratalo',school:'Koulu',kindergarten:'Päiväkoti',pharmacy:'Apteekki',fuel:'Huoltoasema',hospital:'Sairaala',clinic:'Terveysasema',doctors:'Lääkäri',bank:'Pankki',post_office:'Posti',restaurant:'Ravintola',cafe:'Kahvila',library:'Kirjasto',town:'Kaupunki',village:'Kylä'};
  function km(la,lo){var R=6371,r=function(x){return x*Math.PI/180;},a=r(la-LAT),b=r(lo-LON);var h=Math.sin(a/2)*Math.sin(a/2)+Math.cos(r(LAT))*Math.cos(r(la))*Math.sin(b/2)*Math.sin(b/2);return R*2*Math.asin(Math.sqrt(h));}
  var nb=document.getElementById('nearby');
  var q='[out:json][timeout:25];(nwr(around:6000,'+LAT+','+LON+')[shop~"supermarket|convenience|general|department_store"];nwr(around:9000,'+LAT+','+LON+')[amenity~"school|kindergarten|pharmacy|fuel|hospital|clinic|doctors|bank|post_office|restaurant|cafe|library"];nwr(around:14000,'+LAT+','+LON+')[place~"town|village"];);out tags center 120;';
  function run(eps){
    if(!eps.length){if(nb)nb.innerHTML='<span class="muted">Lähipalvelujen haku ei juuri nyt onnistunut.</span>';return;}
    fetch(eps[0],{method:'POST',headers:{'Content-Type':'application/x-www-form-urlencoded'},body:'data='+encodeURIComponent(q)})
      .then(function(r){if(!r.ok)throw 0;return r.json();})
      .then(function(d){render(d.elements||[]);})
      .catch(function(){run(eps.slice(1));});
  }
  function render(els){
    var items=els.map(function(e){var c=e.center||e,t=e.tags||{},k=t.shop||t.amenity||t.place||'';return{n:t.name||'',k:k,lat:c.lat,lon:c.lon,d:(c.lat!=null)?km(c.lat,c.lon):null};})
      .filter(function(x){return x.lat!=null&&x.d!=null&&LABELS[x.k];}).sort(function(a,b){return a.d-b.d;});
    var byCat={};items.forEach(function(x){if(!byCat[x.k])byCat[x.k]=x;});
    items.slice(0,28).forEach(function(x){
      var m=document.createElement('div');m.className='poi'+((x.k==='town'||x.k==='village')?' poi-town':'');
      new maplibregl.Marker({element:m}).setLngLat([x.lon,x.lat]).setPopup(new maplibregl.Popup({offset:14}).setText((LABELS[x.k]||x.k)+(x.n?' · '+x.n:''))).addTo(map);
    });
    if(!nb)return;
    var keys=Object.keys(byCat).sort(function(a,b){return byCat[a].d-byCat[b].d;}).slice(0,9);
    if(!keys.length){nb.innerHTML='<span class="muted">Ei kartoitettuja palveluja lähistöllä — syrjäinen, rauhallinen sijainti.</span>';return;}
    nb.innerHTML='<div class="nbhead">Lähimmät palvelut (OpenStreetMap)</div>'+keys.map(function(k){var x=byCat[k];return '<div class="nb"><span class="nbk">'+(LABELS[k]||k)+(x.n?' · '+x.n:'')+'</span><span class="nbd">'+x.d.toFixed(1).replace('.',',')+' km</span></div>';}).join('');
  }
  map.on('load',function(){run(['https://overpass-api.de/api/interpreter','https://overpass.kumi.systems/api/interpreter']);});
})();
</script>
</body></html>`;
}
