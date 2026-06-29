/**
 * Plane-A visir (fasteignir.visir.is) adapter — Iceland (IS). Iceland is
 * centralized: every licensed `fasteignasali` pushes to the shared agent backend
 * that this open, unauthenticated JSON feed reads from, so one adapter on visir
 * covers the national market (unlike SE's Hemnet+Booli split).
 *
 * The feed is `/api/search` — no auth, JSON, `zip` is a single postcode per
 * request (CSV not supported there), so the crawler fans out per (zip × stype).
 * Every call is wrapped so a failure (429, 5xx, datacenter-IP block, schema
 * drift) returns `{ ok:false, listings:[] }` rather than throwing and breaking
 * the crawl. Volatile params/headers come from `source_config` in D1.
 *
 * Prices arrive in ISK; we convert to EUR at the pack's FX (144 ISK/€).
 */
import { getSourceConfig } from "../db";
import {
  asciiFold,
  normalizeCountry,
  toNumber,
  type NormalizedListing,
} from "../normalize";

export interface VisirQuery {
  /** Single Icelandic postcode (e.g. "103"). The feed accepts ONE zip per call. */
  zip: string;
  /** `"sale"` (default) or `"rent"`. */
  stype?: string;
  page: number;
  /** Page size / per-request hard cap (tested up to 1000). */
  onpage?: number;
}

export interface VisirFetchResult {
  listings: NormalizedListing[];
  found: number;
  ok: boolean;
  error?: string;
}

const DEFAULT_UA =
  "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

/** Live FX from the IS facts pack (2026-06-29). Configurable via source_config. */
const DEFAULT_ISK_PER_EUR = 144;

const sleep = (ms: number): Promise<void> => new Promise((r) => setTimeout(r, ms));

function buildSearchUrl(cfg: Record<string, string>, q: VisirQuery): string {
  const base = cfg["search_url"] ?? "https://fasteignir.visir.is/api/search";
  const params = new URLSearchParams();
  params.set("onpage", String(q.onpage ?? numberOr(cfg["onpage"], 1000)));
  params.set("page", String(q.page));
  params.set("zip", q.zip);
  params.set("stype", q.stype ?? cfg["stype_for_sale"] ?? "sale");
  return `${base}?${params.toString()}`;
}

/**
 * Fetch one page of visir cards for a single postcode. Best-effort: returns
 * `{ ok:false, listings:[] }` on any failure (429, 5xx, block, parse error),
 * retrying with exponential backoff on 429/transient network errors. Never throws.
 */
export async function fetchVisirPage(db: D1Database, q: VisirQuery): Promise<VisirFetchResult> {
  const cfg = await getSourceConfig(db, "visir");
  const ua = cfg["user_agent"] ?? DEFAULT_UA;
  const fx = numberOr(cfg["isk_per_eur"], DEFAULT_ISK_PER_EUR);
  const throttle = numberOr(cfg["rate_limit_ms"], 2500);
  const url = buildSearchUrl(cfg, q);

  await sleep(throttle);

  let lastError = "";
  for (let attempt = 0; attempt < 3; attempt++) {
    try {
      const res = await fetch(url, {
        headers: {
          "User-Agent": ua,
          Accept: "application/json",
          Referer: cfg["referer"] ?? "https://fasteignir.visir.is/",
        },
      });
      if (res.status === 429 || res.status >= 500) {
        lastError = `visir HTTP ${res.status}`;
        await sleep(throttle * Math.pow(2, attempt + 1));
        continue;
      }
      if (!res.ok) {
        return { listings: [], found: 0, ok: false, error: `visir HTTP ${res.status}` };
      }
      const body = (await res.json()) as unknown;
      const cards = extractCards(body);
      const listings: NormalizedListing[] = [];
      for (const card of cards) {
        const row = normalizeVisirCard(card, fx);
        if (row.portal_listing_id !== "") listings.push(row);
      }
      return { listings, found: extractFound(body, listings.length), ok: true };
    } catch (err) {
      lastError = String(err);
      await sleep(throttle * Math.pow(2, attempt + 1));
    }
  }
  console.warn("visir fetch failed", lastError);
  return { listings: [], found: 0, ok: false, error: lastError };
}

/** The feed may return a bare array or wrap the cards under a known key. */
function extractCards(body: unknown): unknown[] {
  if (Array.isArray(body)) return body;
  if (body == null || typeof body !== "object") return [];
  const o = body as Record<string, unknown>;
  for (const key of ["results", "properties", "data", "items", "hits", "listings"]) {
    const v = o[key];
    if (Array.isArray(v)) return v;
  }
  return [];
}

function extractFound(body: unknown, fallback: number): number {
  if (body == null || typeof body !== "object") return fallback;
  const o = body as Record<string, unknown>;
  for (const key of ["total", "found", "count", "totalCount", "hits_total"]) {
    const v = o[key];
    const n = toNumber(v);
    if (n != null) return Math.round(n);
  }
  return fallback;
}

/**
 * visir `category` (Icelandic property-type label) → kontu normalized enum, per
 * the IS facts pack §8. Non-residential categories return null so the crawler
 * can drop them. Matches on a folded substring so RE/MAX-style variant suffixes
 * ("Fjölbýlishús með lyftu") still classify.
 */
function mapVisirCategory(raw: unknown): string | null {
  const s = asciiFold(raw);
  if (s === "") return null;
  if (/atvinnuhusnaedi|skrifstofuhusnaedi|verslunarhusnaedi|idnadarhusnaedi/.test(s)) return null;
  if (/einbylishus|einbyli/.test(s)) return "detached_house";
  if (/parhus/.test(s)) return "semi_detached";
  if (/radhus/.test(s)) return "terraced_house";
  if (/(sumarhus|sumarbustadur|sumarbustad|orlofshus)/.test(s)) return "cottage";
  if (/(jord|byli|jardir)/.test(s)) return "farm";
  if (/(lod|land)/.test(s)) return "plot";
  if (/(fjolbylishus|tvibyli|thribyli|fjorbyli|haed|ibud)/.test(s)) return "apartment";
  return null;
}

/**
 * Heating type from free text (visir carries NO structured heating field; this is
 * a fallback for when the description leaks into a card). IS facts pack §8 → kontu
 * `heating_type`. Heating is near-uniform cheap geothermal, so default is null
 * (unknown) rather than presuming district heat from a search card.
 */
function mapVisirHeating(raw: unknown): string | null {
  const s = asciiFold(raw);
  if (s === "") return null;
  if (/(hitaveita|heitt vatn|jardvarm|jardhit)/.test(s)) return "district_heat";
  if (/(rafhitun|rafmagnskynding|rafkynding|rafmagnshitun)/.test(s)) return "direct_electric";
  if (/(oliukynding|oliuhitun|olia)/.test(s)) return "oil";
  return null;
}

/**
 * Shore/waterfront from free text (NOT a structured field on any IS portal; the
 * search card has no shore field). IS facts pack §8 → kontu `shore`. Text-derived,
 * lower confidence; returns null on a search card that carries no descriptive text.
 */
function mapVisirShore(raw: unknown): string | null {
  const s = asciiFold(raw);
  if (s === "") return null;
  if (/(sjavarlod|vid vatnid|vatnslod|sjavarjord|vid sjoinn|strandlengja)/.test(s)) return "oma_ranta";
  if (/(sjavarutsyni|vatnsutsyni|sea view)/.test(s)) return "sea_view";
  return null;
}

/**
 * Condition class from free text → kontu `condition_class`. visir cards carry no
 * structured condition field; returns null unless the text asserts one. Reuses the
 * Finnish vocabulary loosely (Icelandic listings rarely state a class on the card).
 */
function mapVisirCondition(raw: unknown): string | null {
  const s = asciiFold(raw);
  if (s === "") return null;
  if (/(nybygg|nytt|nybyggt|nybygging)/.test(s)) return "uudiskohde";
  return null;
}

/**
 * Map one visir search-card object into a NormalizedListing. Card keys are exact
 * (id, street_name, price, size, category, rooms, latitude/longitude) but every
 * access is defensive — the source schema drifts and fields go missing. Price is
 * an ISK string converted to EUR at `fx` (ISK per €). Heating/shore are not
 * structured on the card → null here; parse them from the detail `Lýsing` later.
 */
function normalizeVisirCard(card: unknown, fx: number): NormalizedListing {
  const c = (card ?? {}) as Record<string, unknown>;
  const id = firstString(c["id"], c["property_id"], c["propertyId"]) ?? "";
  const url = firstString(c["url"], c["link"]) ??
    (id ? `https://fasteignir.visir.is/property/${id}` : "https://fasteignir.visir.is/");

  const category = firstString(c["category"], c["type"], c["property_type"]);
  const address = buildAddress(c);
  const zipObj = (c["zip"] ?? null) as Record<string, unknown> | null;

  const priceIsk = toNumber(firstString(c["price"], c["sale_price"], c["asking_price"]));
  const priceEur = positiveOrNull(priceIsk) != null && fx > 0
    ? Math.round((priceIsk as number) / fx)
    : null;

  const row: NormalizedListing = {
    portal: "visir",
    portal_listing_id: id,
    url,
    country: normalizeCountry("IS") ?? "IS",

    property_type: mapVisirCategory(category),
    holding_form: null,
    kiinteistotunnus: firstString(c["fastanumer"], c["fastanr"], c["property_number"]),
    address,
    municipality: firstString(
      zipObj ? zipObj["town"] : undefined,
      c["town"],
      c["city"],
      c["municipality"],
    ),
    postal_code: firstString(zipObj ? zipObj["zip"] : undefined, c["zip"], c["postcode"], c["postal_code"]),
    district: firstString(c["district"], c["area"], c["neighbourhood"]),
    lat: toNumber(firstString(c["latitude"], c["lat"], get(c, "coordinates.lat"))),
    lon: toNumber(firstString(c["longitude"], c["lon"], c["lng"], get(c, "coordinates.lon"))),

    price_eur: priceEur,
    debt_free_price_eur: null,
    debt_share_eur: null,
    price_per_m2: null,
    maintenance_charge_eur: null,
    financing_charge_eur: null,
    ground_rent_eur_yr: null,

    living_area_m2: toNumber(firstString(c["size"], c["area"], c["living_area"])),
    total_area_m2: null,
    plot_area_m2: null,
    room_count: toNumber(firstString(c["rooms"], c["room_count"])),
    room_layout: null,
    floors: null,

    year_built: toInt(firstString(c["build_year"], c["byggt"], c["year"], c["construction_year"])),
    occupancy_year: null,
    roof_year: null,
    pipes_renovated_year: null,
    water_body: null,
    kiinteistovero_eur_yr: null,
    electricity_eur_yr: null,
    condition_class: mapVisirCondition(firstString(c["description"], c["lysing"])),
    inspection_status: null,
    frame_material: null,
    facade_material: null,
    roof_material: null,
    energy_class: null,
    e_value: null,
    risk_structures: [],

    plot_ownership: null,
    lease_end_year: null,
    shore: mapVisirShore(firstString(c["description"], c["lysing"], address)),
    shore_sauna: null,

    heating_type: mapVisirHeating(firstString(c["description"], c["lysing"])),
    heat_distribution: null,
    water_supply: null,
    sewer_system: null,
    broadband: null,
    sauna: null,
    parking: firstString(c["bilskur"], c["garage"], c["parking"]),
    road_access: null,
    intended_use: null,
    zoning_status: null,
    description: firstString(c["description"], c["lysing"]),

    status: mapVisirStatus(priceIsk, c["status"]),
    raw_json: JSON.stringify(card ?? {}),
  };
  return row;
}

/** Join street name + number when the number is a separate field. */
function buildAddress(c: Record<string, unknown>): string | null {
  const street = firstString(c["street_name"], c["street"], c["address"]);
  const num = firstString(c["street_number"]);
  if (street && num && !street.includes(num)) return `${street} ${num}`;
  return street;
}

/**
 * Per §8: price `"0"` = Tilboð / price-on-application (POA), not a real asking
 * price, but the listing is still on the market. We keep status "active" for both
 * (the price simply maps to null); only an explicit sold/reserved marker changes it.
 */
function mapVisirStatus(priceIsk: number | null, status: unknown): string {
  const s = asciiFold(status);
  if (/(sold|seld|selt|fra ferli)/.test(s)) return "sold";
  if (/(reserved|fratekid|fratekin|tilbod samthykkt)/.test(s)) return "reserved";
  if (/(withdrawn|afskrad|tekin ur solu)/.test(s)) return "withdrawn";
  void priceIsk;
  return "active";
}

function numberOr(s: string | undefined, def: number): number {
  const n = s == null ? NaN : Number(s);
  return Number.isFinite(n) ? n : def;
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

function toInt(v: unknown): number | null {
  const n = toNumber(v);
  return n == null ? null : Math.round(n);
}

function positiveOrNull(n: number | null): number | null {
  return n != null && n > 0 ? n : null;
}

function get(obj: unknown, path: string): unknown {
  if (obj == null || typeof obj !== "object") return undefined;
  let cur: unknown = obj;
  for (const key of path.split(".")) {
    if (cur == null || typeof cur !== "object") return undefined;
    cur = (cur as Record<string, unknown>)[key];
  }
  return cur;
}
