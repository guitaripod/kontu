/**
 * Plane-A FINN.no (Norway) adapter. FINN is a Next.js app whose
 * `…/realestate/<vertical>/search.html` pages are robots-allowed and ship a
 * hydration blob (`__NEXT_DATA__` → `props.pageProps`) that mirrors the FI
 * Oikotie `/api/cards` shape. We drive the public search URL with a normal
 * desktop-Chrome User-Agent, parse the embedded JSON, and map each card.
 *
 * Disposable by design (same posture as oikotie.ts / etuovi.ts): every call is
 * wrapped so a block / bot-challenge / schema-drift logs and returns
 * `{ ok:false }` rather than throwing and breaking the crawl. All volatile bits
 * (search URL, UA, throttle, and the UNVERIFIED numeric filter codes) come from
 * `source_config` in D1 with hardcoded fallbacks; nothing secret lives here.
 */
import { parse } from "node-html-parser";
import { getSourceConfig } from "../db";
import {
  asciiFold,
  extractRiskStructures,
  normalizeConditionClass,
  normalizeEnergyClass,
  normalizeHeatingType,
  normalizeHoldingForm,
  normalizePlotOwnership,
  normalizePropertyType,
  normalizeShore,
  toNumber,
  type NormalizedListing,
} from "../normalize";

/** FX used to convert NOK → EUR for the normalized `*_eur` fields. */
const NOK_PER_EUR = 11.2;

const DEFAULT_UA =
  "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

const DEFAULT_HOMES_URL = "https://www.finn.no/realestate/homes/search.html";
const DEFAULT_LEISURE_URL = "https://www.finn.no/realestate/leisuresale/search.html";

const sleep = (ms: number): Promise<void> => new Promise((r) => setTimeout(r, ms));

/**
 * UNVERIFIED numeric filter codes. FINN's `property_type`/`ownership_type`/
 * `facilities` params take undocumented numeric codes that drift; these are
 * best-guess placeholders to be confirmed via live DevTools on a residential IP
 * and then moved into `source_config` (keys `property_type_codes`,
 * `ownership_type_codes`, `facility_codes`). The fetcher prefers source_config
 * over these constants whenever present.
 */
const UNVERIFIED_PROPERTY_TYPE_CODES: Record<string, string> = {
  enebolig: "1",
  tomannsbolig: "2",
  rekkehus: "3",
  leilighet: "4",
  gardsbruk: "5",
  hytte: "6",
};

const UNVERIFIED_OWNERSHIP_TYPE_CODES: Record<string, string> = {
  selveier: "3",
  andel: "1",
  aksje: "2",
};

const UNVERIFIED_FACILITY_CODES: Record<string, string> = {
  strandlinje: "1",
};

/** Which FINN vertical to drive: houses+apartments, or cabins (the lakeside lane). */
export type FinnVertical = "homes" | "leisuresale";

export interface FinnQuery {
  /** FINN-internal dotted location taxonomy (e.g. `1.20012.20196`), repeatable. */
  locations?: string[];
  /** Plain municipality name — passed through `q` as a fallback when no code. */
  municipality?: string;
  priceMin?: number;
  priceMax?: number;
  areaMin?: number;
  areaMax?: number;
  yearMin?: number;
  yearMax?: number;
  /** Normalized property-type token(s): enebolig/leilighet/hytte/… */
  propertyTypes?: string[];
  /** Normalized ownership token(s): selveier/andel/aksje. */
  ownershipTypes?: string[];
  /** Facility token(s); `strandlinje` is the shoreline (waterfront) filter. */
  facilities?: string[];
  /** 1-based page index (FINN paginates `&page=N`). */
  page: number;
  /** Vertical to search; defaults to `homes`. */
  vertical?: FinnVertical;
}

export interface FinnFetchResult {
  listings: NormalizedListing[];
  found: number;
  ok: boolean;
  error?: string;
}

function buildSearchUrl(cfg: Record<string, string>, q: FinnQuery): string {
  const vertical: FinnVertical = q.vertical ?? "homes";
  const base =
    vertical === "leisuresale"
      ? cfg["leisure_search_url"] ?? DEFAULT_LEISURE_URL
      : cfg["search_url"] ?? DEFAULT_HOMES_URL;

  const params = new URLSearchParams();
  for (const loc of q.locations ?? []) {
    if (loc) params.append("location", loc);
  }
  if (q.municipality && (q.locations ?? []).length === 0) {
    params.set("q", q.municipality);
  }
  if (q.priceMin != null) params.set("price_from", String(q.priceMin));
  if (q.priceMax != null) params.set("price_to", String(q.priceMax));
  if (q.areaMin != null) params.set("area_from", String(q.areaMin));
  if (q.areaMax != null) params.set("area_to", String(q.areaMax));
  if (q.yearMin != null) params.set("year_from", String(q.yearMin));
  if (q.yearMax != null) params.set("year_to", String(q.yearMax));

  const ptCodes = parseJsonObject(cfg["property_type_codes"]) ?? UNVERIFIED_PROPERTY_TYPE_CODES;
  for (const t of q.propertyTypes ?? []) {
    const code = ptCodes[asciiFold(t)];
    if (code != null) params.append("property_type", String(code));
  }

  const otCodes = parseJsonObject(cfg["ownership_type_codes"]) ?? UNVERIFIED_OWNERSHIP_TYPE_CODES;
  for (const o of q.ownershipTypes ?? []) {
    const code = otCodes[asciiFold(o)];
    if (code != null) params.append("ownership_type", String(code));
  }

  const facCodes = parseJsonObject(cfg["facility_codes"]) ?? UNVERIFIED_FACILITY_CODES;
  for (const f of q.facilities ?? []) {
    const code = facCodes[asciiFold(f)];
    if (code != null) params.append("facilities", String(code));
  }

  if (q.page > 1) params.set("page", String(q.page));

  const sort = cfg["sort"];
  if (sort) params.set("sort", sort);

  const qs = params.toString();
  return qs ? `${base}?${qs}` : base;
}

/**
 * Fetch one FINN search page with retry/backoff on 429/5xx. Best-effort:
 * returns `{ ok:false, listings:[] }` on any failure (block, challenge, parse
 * miss, schema drift). Never throws.
 */
export async function fetchFinnPage(db: D1Database, q: FinnQuery): Promise<FinnFetchResult> {
  const cfg = await getSourceConfig(db, "finn");
  const ua = cfg["user_agent"] ?? DEFAULT_UA;
  const throttle = numberOr(cfg["rate_limit_ms"], 1200);
  const url = buildSearchUrl(cfg, q);

  await sleep(throttle);

  let lastError = "";
  for (let attempt = 0; attempt < 3; attempt++) {
    try {
      const res = await fetch(url, {
        headers: {
          "User-Agent": ua,
          Accept: "text/html,application/xhtml+xml,application/xml;q=0.9,application/json;q=0.8,*/*;q=0.7",
          "Accept-Language": "nb-NO,nb;q=0.9,en;q=0.7",
          Referer: "https://www.finn.no/realestate/",
        },
      });
      if (res.status === 429 || res.status >= 500) {
        lastError = `finn HTTP ${res.status}`;
        await sleep(throttle * Math.pow(2, attempt + 1));
        continue;
      }
      if (!res.ok) {
        return { listings: [], found: 0, ok: false, error: `finn HTTP ${res.status}` };
      }
      const html = await res.text();
      const blob = extractNextData(html);
      if (blob == null) {
        return {
          listings: [],
          found: 0,
          ok: false,
          error: "finn: __NEXT_DATA__ not found (block, challenge, or schema drift)",
        };
      }
      const docs = extractDocs(blob);
      const found = extractFound(blob, docs.length);
      return {
        listings: docs.map((d) => normalizeFinnCard(d, q.vertical ?? "homes")),
        found,
        ok: true,
      };
    } catch (err) {
      lastError = String(err);
      await sleep(throttle * Math.pow(2, attempt + 1));
    }
  }
  console.warn("finn fetch failed", lastError);
  return { listings: [], found: 0, ok: false, error: lastError };
}

/**
 * Pull the `__NEXT_DATA__` JSON blob from a FINN search page. FINN is Next.js,
 * so the search results hydrate from `<script id="__NEXT_DATA__" …>{…}</script>`.
 * Returns `null` if the script is absent (page block / challenge / drift).
 */
function extractNextData(html: string): Record<string, unknown> | null {
  try {
    const root = parse(html);
    const el = root.querySelector("script#__NEXT_DATA__");
    const text = el?.text;
    if (text && text.trim() !== "") {
      const parsed = JSON.parse(text);
      if (parsed && typeof parsed === "object") return parsed as Record<string, unknown>;
    }
  } catch {
    /* fall through to regex */
  }
  const m = html.match(
    /<script[^>]*id=["']__NEXT_DATA__["'][^>]*>([\s\S]*?)<\/script>/i,
  );
  if (m && m[1]) {
    try {
      const parsed = JSON.parse(m[1]);
      if (parsed && typeof parsed === "object") return parsed as Record<string, unknown>;
    } catch {
      /* give up */
    }
  }
  return null;
}

/**
 * Locate the array of result cards inside the hydration blob. The route to the
 * search result list drifts (`props.pageProps.search.docs`,
 * `…results.docs`, …) so we probe known paths and, failing that, recursively
 * find the largest array of finnkode-bearing objects.
 */
function extractDocs(blob: Record<string, unknown>): Array<Record<string, unknown>> {
  const candidatePaths = [
    "props.pageProps.search.docs",
    "props.pageProps.search.results.docs",
    "props.pageProps.searchData.docs",
    "props.pageProps.results.docs",
    "props.pageProps.docs",
    "props.pageProps.initialState.search.docs",
  ];
  for (const path of candidatePaths) {
    const v = getPath(blob, path);
    const arr = asCardArray(v);
    if (arr.length) return arr;
  }
  const found = deepFindCards(blob, 0);
  return found ?? [];
}

function extractFound(blob: Record<string, unknown>, fallback: number): number {
  for (const path of [
    "props.pageProps.search.metadata.numResults",
    "props.pageProps.search.metadata.result_size.match_count",
    "props.pageProps.search.totalResults",
    "props.pageProps.metadata.numResults",
    "props.pageProps.searchData.metadata.numResults",
  ]) {
    const v = getPath(blob, path);
    if (typeof v === "number" && Number.isFinite(v)) return v;
    const n = toNumber(v);
    if (n != null) return Math.round(n);
  }
  return fallback;
}

function asCardArray(v: unknown): Array<Record<string, unknown>> {
  if (!Array.isArray(v)) return [];
  return v.filter(
    (x): x is Record<string, unknown> => x != null && typeof x === "object" && !Array.isArray(x),
  );
}

/** True when an object smells like a FINN ad card (carries a finnkode/ad id). */
function looksLikeCard(o: Record<string, unknown>): boolean {
  return (
    "ad_id" in o || "finnkode" in o || "id" in o || "adId" in o || "ad_link" in o || "canonical_url" in o
  );
}

/** Depth-bounded search for the largest array of card-shaped objects. */
function deepFindCards(
  node: unknown,
  depth: number,
): Array<Record<string, unknown>> | null {
  if (depth > 8 || node == null || typeof node !== "object") return null;
  if (Array.isArray(node)) {
    const cards = asCardArray(node);
    if (cards.length && cards.some(looksLikeCard)) return cards;
    let best: Array<Record<string, unknown>> | null = null;
    for (const item of node) {
      const found = deepFindCards(item, depth + 1);
      if (found && (!best || found.length > best.length)) best = found;
    }
    return best;
  }
  let best: Array<Record<string, unknown>> | null = null;
  for (const val of Object.values(node as Record<string, unknown>)) {
    const found = deepFindCards(val, depth + 1);
    if (found && (!best || found.length > best.length)) best = found;
  }
  return best;
}

/**
 * Map one FINN search card into a NormalizedListing. The card schema drifts and
 * many house facts (heating, water/sewer, shore) live only as free text, so
 * every access is defensive and the function never throws. Prices are NOK and
 * converted to EUR at the module FX rate.
 */
function normalizeFinnCard(card: unknown, vertical: FinnVertical): NormalizedListing {
  const c = (card ?? {}) as Record<string, unknown>;

  const id =
    firstString(c["ad_id"], c["finnkode"], c["adId"], c["id"]) ?? "";
  const url = resolveUrl(c, id);

  const heading = firstString(c["heading"], c["title"]) ?? "";
  const description =
    firstString(c["description"], c["body"], c["text"], c["preview"]) ?? "";
  const freeText = [heading, description, joinStrings(c["facilities"])]
    .filter((s) => s !== "")
    .join(" ");

  const location = (c["location"] ?? null) as Record<string, unknown> | null;
  const coordinates = (c["coordinates"] ?? c["coordinate"] ?? null) as
    | Record<string, unknown>
    | null;

  const propertyTypeRaw = firstString(
    c["property_type"],
    c["propertyType"],
    get(c, "property_type.name"),
    vertical === "leisuresale" ? "hytte" : null,
    heading,
  );
  const ownershipRaw = firstString(c["ownership_type"], c["owner_type"], c["eierform"]);

  const priceNok = pickNok(
    c["price"],
    get(c, "price.amount"),
    get(c, "price_suggestion.amount"),
    c["price_suggestion"],
    c["prisantydning"],
  );
  const totalNok = pickNok(
    get(c, "price_total.amount"),
    c["price_total"],
    c["totalpris"],
    c["total_price"],
  );
  const debtNok = pickNok(get(c, "price_shared_cost.amount"), c["fellesgjeld"], c["shared_debt"]);

  const livingArea = toNumber(
    firstString(get(c, "area_range.size_from"), c["area_size"], c["living_area"], c["area"], c["usable_size"]),
  );
  const plotArea = toNumber(firstString(c["plot_area"], c["tomteareal"], get(c, "plot.area")));

  const energyRaw = firstString(c["energy_label"], get(c, "energy_label.code"), c["energimerking"]);

  const row: NormalizedListing = {
    portal: "finn",
    portal_listing_id: id,
    url,
    country: "NO",

    property_type: mapFinnPropertyType(propertyTypeRaw, vertical),
    holding_form: mapFinnHoldingForm(ownershipRaw),
    kiinteistotunnus: null,
    address: firstString(
      c["address"],
      get(c, "location.address"),
      location ? firstString(location["street_address"], location["address"]) : null,
    ),
    municipality: firstString(
      location ? firstString(location["municipality"], location["city"]) : null,
      c["municipality"],
      c["city"],
    ),
    postal_code: firstString(
      location ? firstString(location["postal_code"], location["postalCode"]) : null,
      c["postal_code"],
    ),
    district: firstString(
      location ? firstString(location["district"], location["area"]) : null,
      c["district"],
    ),
    lat: pickCoord(coordinates, ["lat", "latitude"], c, ["lat", "latitude"]),
    lon: pickCoord(coordinates, ["lon", "lng", "longitude"], c, ["lon", "lng", "longitude"]),

    price_eur: nokToEur(priceNok),
    debt_free_price_eur: nokToEur(totalNok),
    debt_share_eur: nokToEur(debtNok),
    price_per_m2: ratioOrNull(nokToEur(priceNok), livingArea),
    maintenance_charge_eur: nokToEur(
      pickNok(get(c, "price_shared_cost_monthly.amount"), c["felleskostnader"], c["shared_cost"]),
    ),
    financing_charge_eur: null,
    ground_rent_eur_yr: nokToEur(pickNok(c["festeavgift"], c["ground_rent"])),

    living_area_m2: livingArea,
    total_area_m2: toNumber(firstString(c["gross_area"], c["bra"], c["total_area"])),
    plot_area_m2: plotArea,
    room_count: toNumber(firstString(c["number_of_bedrooms"], c["bedrooms"], c["rooms"], c["number_of_rooms"])),
    room_layout: null,
    floors: toNumber(firstString(c["floor"], c["number_of_floors"], c["floors"])),

    year_built: toInt(firstString(c["construction_year"], c["build_year"], c["byggear"], c["year_built"])),
    occupancy_year: null,
    roof_year: null,
    pipes_renovated_year: null,
    water_body: null,
    kiinteistovero_eur_yr: nokToEur(pickNok(c["eiendomsskatt"], c["property_tax"])),
    electricity_eur_yr: null,
    condition_class: normalizeConditionClass(firstString(c["condition"], c["tilstand"])) ?? null,
    inspection_status: extractTgStatus(freeText),
    frame_material: null,
    facade_material: firstString(c["facade_material"], c["fasade"]),
    roof_material: firstString(c["roof_material"], c["taktekking"]),
    energy_class: normalizeEnergyClass(energyRaw),
    e_value: null,
    risk_structures: extractRiskStructures(freeText),

    plot_ownership: mapFinnPlotOwnership(firstString(c["tomtetype"], c["plot_ownership"], freeText)),
    lease_end_year: null,
    shore: mapFinnShore(c, freeText),
    shore_sauna: null,

    heating_type: normalizeHeatingType(mapFinnHeating(freeText)),
    heat_distribution: /vannb[åa]ren|vannbaren/i.test(freeText) ? "vesikiertoinen" : null,
    water_supply: mapFinnWater(freeText),
    sewer_system: mapFinnSewer(freeText),
    broadband: null,
    sauna: null,
    parking: firstString(c["parking"]),
    road_access: null,
    intended_use: null,
    zoning_status: null,
    description: description || null,

    status: mapFinnStatus(c),
    raw_json: JSON.stringify(card ?? {}),
  };
  return row;
}

function resolveUrl(c: Record<string, unknown>, id: string): string {
  const direct = firstString(c["canonical_url"], get(c, "ad_link"), c["url"], get(c, "links.0.href"));
  if (direct) {
    if (direct.startsWith("//")) return `https:${direct}`;
    if (direct.startsWith("/")) return `https://www.finn.no${direct}`;
    if (/^https?:\/\//i.test(direct)) return direct;
  }
  return id
    ? `https://www.finn.no/realestate/homes/ad.html?finnkode=${id}`
    : "https://www.finn.no/realestate/";
}

/** NOK amount → EUR (rounded), null when the source figure is missing/non-positive. */
function nokToEur(nok: number | null): number | null {
  if (nok == null || nok <= 0) return null;
  return Math.round(nok / NOK_PER_EUR);
}

/** Coerce a FINN money field (number, `{amount}`, or "1 234 567 kr") to a NOK number. */
function pickNok(...vals: unknown[]): number | null {
  for (const v of vals) {
    if (v == null) continue;
    if (typeof v === "number") {
      if (Number.isFinite(v) && v > 0) return v;
      continue;
    }
    if (typeof v === "object" && !Array.isArray(v)) {
      const amount = (v as Record<string, unknown>)["amount"];
      const n = toNumber(amount);
      if (n != null && n > 0) return n;
      continue;
    }
    const n = toNumber(v);
    if (n != null && n > 0) return n;
  }
  return null;
}

function ratioOrNull(numerator: number | null, denominator: number | null): number | null {
  if (numerator == null || denominator == null || denominator <= 0) return null;
  return Math.round(numerator / denominator);
}

function pickCoord(
  coord: Record<string, unknown> | null,
  coordKeys: string[],
  c: Record<string, unknown>,
  cardKeys: string[],
): number | null {
  if (coord) {
    for (const k of coordKeys) {
      const n = toNumber(coord[k]);
      if (n != null && n !== 0) return n;
    }
  }
  for (const k of cardKeys) {
    const n = toNumber(c[k]);
    if (n != null && n !== 0) return n;
  }
  return null;
}

/** FINN `property_type` (`Enebolig`…) / vertical → kontu type, via normalize.ts. */
function mapFinnPropertyType(raw: string | null, vertical: FinnVertical): string | null {
  const s = asciiFold(raw);
  if (s === "") return vertical === "leisuresale" ? "mokki" : null;
  if (/enebolig/.test(s)) return "omakotitalo";
  if (/tomannsbolig|vertikaldelt|2-mannsbolig/.test(s)) return "paritalo";
  if (/rekkehus/.test(s)) return "rivitalo";
  if (/leilighet/.test(s)) return "kerrostalo";
  if (/(gardsbruk|smabruk|gard\b)/.test(s)) return "maatila";
  if (/(hytte|fritidsbolig|fritid)/.test(s)) return "mokki";
  if (vertical === "leisuresale") return "mokki";
  return normalizePropertyType(raw);
}

/** FINN `eierform` (`Selveier`/`Andel`/`Aksje`) → kontu holding form. */
function mapFinnHoldingForm(raw: string | null): string | null {
  const s = asciiFold(raw);
  if (s === "") return null;
  if (/selveier|eier\b|fast eiendom/.test(s)) return "kiinteisto";
  if (/(andel|borettslag|aksje|aksjeleilighet|obligasjon)/.test(s)) return "asunto_osake";
  return normalizeHoldingForm(raw);
}

/** FINN `Eid`/`Festet tomt` → kontu plot ownership. */
function mapFinnPlotOwnership(raw: string | null): string | null {
  const s = asciiFold(raw);
  if (s === "") return null;
  if (/festet|feste\b|festeavgift/.test(s)) return "vuokra";
  if (/eiet|eid tomt|selveiet/.test(s)) return "oma";
  return normalizePlotOwnership(raw);
}

/** Map free-text Norwegian heating prose to a FI heating token for normalize.ts. */
function mapFinnHeating(text: string): string | null {
  const s = asciiFold(text);
  if (s === "") return null;
  if (/fjernvarme/.test(s)) return "kaukolampo";
  if (/(bergvarme|jordvarme|vaeske-til-vann|vaske-til-vann|geovarme)/.test(s)) return "maalampo";
  if (/luft-til-vann|luft til vann/.test(s)) return "ivlp";
  if (/(varmepumpe|luft-til-luft|luft til luft)/.test(s)) return "ilmalampopumppu";
  if (/(oljefyr|parafin|mineralolje)/.test(s)) return "oljy";
  if (/(vedfyring|peis|ildsted|pelletskamin|vedovn|kaminovn)/.test(s)) return "puu";
  if (/(elektrisk|panelovn|varmekabler|elvarme)/.test(s)) return "sahko";
  if (/vannb[åa]ren|vannbaren/.test(s)) return "vesikiertoinen";
  return null;
}

function mapFinnWater(text: string): string | null {
  const s = asciiFold(text);
  if (s === "") return null;
  if (/(egen bronn|borebronn|privat vann|bronn\b)/.test(s)) return "oma kaivo";
  if (/(offentlig vann|kommunalt vann|kommunal vann)/.test(s)) return "kunnallinen";
  return null;
}

function mapFinnSewer(text: string): string | null {
  const s = asciiFold(text);
  if (s === "") return null;
  if (/minirenseanlegg/.test(s)) return "pienpuhdistamo";
  if (/(septik|slamavskiller|septiktank)/.test(s)) return "sakokaivo";
  if (/(offentlig avlop|kommunalt avlop|kommunal avlop)/.test(s)) return "kunnallinen viemari";
  return null;
}

/** Shore: the `Strandlinje` facility or shore-text terms → kontu `oma_ranta`. */
function mapFinnShore(c: Record<string, unknown>, text: string): string | null {
  const facilities = joinStrings(c["facilities"]);
  const s = asciiFold(`${facilities} ${text}`);
  if (/(strandlinje|strandtomt|sjotomt|egen strand|vannkant|egen strandlinje)/.test(s)) {
    return "oma_ranta";
  }
  if (/(batplass|naust|strandsone)/.test(s)) return "rantaoikeus";
  const viaNormalize = normalizeShore(s);
  return viaNormalize;
}

/** Surface the avhendingslova condition-report grade as the inspection status. */
function extractTgStatus(text: string): string | null {
  const m = text.match(/\bTG\s?([0-3])\b/i);
  if (m) return `TG${m[1]}`;
  if (/tilstandsrapport/i.test(text)) return "tilstandsrapport";
  return null;
}

function mapFinnStatus(c: Record<string, unknown>): string {
  const flags = [
    firstString(c["status"]),
    firstString(c["ad_status"]),
    firstString(c["state"]),
    joinStrings(c["labels"]),
    joinStrings(c["flags"]),
  ]
    .filter((s) => s !== "")
    .join(" ");
  const s = asciiFold(flags);
  if (/solgt|sold/.test(s)) return "sold";
  if (/(reservert|under bud|budaksept)/.test(s)) return "reserved";
  if (/(inaktiv|slettet|trukket|utlopt|expired|withdrawn)/.test(s)) return "withdrawn";
  return "active";
}

function firstString(...vals: unknown[]): string | null {
  for (const v of vals) {
    if (typeof v === "string") {
      const t = v.trim();
      if (t !== "") return t;
    } else if (typeof v === "number" && Number.isFinite(v)) {
      return String(v);
    }
  }
  return null;
}

function joinStrings(v: unknown): string {
  if (Array.isArray(v)) {
    return v
      .map((x) => (typeof x === "string" ? x : x && typeof x === "object" ? firstString((x as Record<string, unknown>)["name"], (x as Record<string, unknown>)["label"], (x as Record<string, unknown>)["value"]) : null))
      .filter((x): x is string => x != null)
      .join(" ");
  }
  return typeof v === "string" ? v : "";
}

function get(obj: unknown, path: string): unknown {
  return getPath(obj, path);
}

function getPath(obj: unknown, path: string): unknown {
  if (obj == null || typeof obj !== "object") return undefined;
  let cur: unknown = obj;
  for (const key of path.split(".")) {
    if (cur == null || typeof cur !== "object") return undefined;
    cur = (cur as Record<string, unknown>)[key];
  }
  return cur;
}

function toInt(v: unknown): number | null {
  const n = toNumber(v);
  return n == null ? null : Math.round(n);
}

function numberOr(s: string | undefined, def: number): number {
  const n = s == null ? NaN : Number(s);
  return Number.isFinite(n) ? n : def;
}

function parseJsonObject(s: string | undefined): Record<string, string> | null {
  if (!s) return null;
  try {
    const v = JSON.parse(s);
    if (v && typeof v === "object" && !Array.isArray(v)) {
      const out: Record<string, string> = {};
      for (const [k, val] of Object.entries(v as Record<string, unknown>)) {
        out[asciiFold(k)] = String(val);
      }
      return out;
    }
    return null;
  } catch {
    return null;
  }
}
