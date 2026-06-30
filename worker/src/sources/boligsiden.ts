/**
 * Plane-A Boligsiden (DK) adapter. Hits the OPEN, unauthenticated JSON API
 * `GET https://api.boligsiden.dk/search/cases` with a normal desktop Chrome
 * User-Agent + `Accept: application/json` (no auth/cookie/referer needed), the
 * cleanest of the four Nordic targets. Throttles, backs off on 429/5xx, and is
 * fully wrapped so failures never break the crawl (returns `{ ok:false }`, never
 * throws). All volatile params/enum maps come from `source_config` in D1 so they
 * can drift without a redeploy. DKK→EUR via the ERM-II peg 7.46 DKK/EUR.
 */
import { getSourceConfig } from "../db";
import {
  asciiFold,
  extractRiskStructures,
  normalizeCountry,
  normalizeEnergyClass,
  type NormalizedListing,
} from "../normalize";

export interface BoligsidenQuery {
  /** Boligsiden municipality slug(s) (e.g. "koebenhavn"), repeatable. */
  municipalities?: string[];
  /** Postal codes, repeatable. */
  zipCodes?: string[];
  /** `addressTypes` slugs (e.g. "villa", "holiday house"), repeatable. */
  addressTypes?: string[];
  priceMin?: number;
  priceMax?: number;
  areaMin?: number;
  areaMax?: number;
  /** `price` | `timeOnMarket` | `createdAt` | `date` | `random`. */
  sort?: string;
  /** 1-indexed. */
  page: number;
  perPage?: number;
}

export interface BoligsidenFetchResult {
  listings: NormalizedListing[];
  found: number;
  ok: boolean;
  error?: string;
}

const DEFAULT_UA =
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

const DEFAULT_SEARCH_URL = "https://api.boligsiden.dk/search/cases";
const DEFAULT_PER_PAGE = 50;

/** DKK per EUR (ERM-II central peg; DKK floats ±2.25%, treat as fixed). */
const DKK_PER_EUR = 7.46;

const sleep = (ms: number): Promise<void> => new Promise((r) => setTimeout(r, ms));

function buildUrl(cfg: Record<string, string>, q: BoligsidenQuery): string {
  const base = cfg["search_url"] ?? DEFAULT_SEARCH_URL;
  const params = new URLSearchParams();

  for (const slug of q.addressTypes ?? []) {
    if (slug) params.append("addressTypes", slug);
  }
  for (const m of q.municipalities ?? []) {
    if (m) params.append("municipalities", m);
  }
  for (const z of q.zipCodes ?? []) {
    if (z) params.append("zipCodes", z);
  }
  if (q.priceMin != null) params.set("priceMin", String(q.priceMin));
  if (q.priceMax != null) params.set("priceMax", String(q.priceMax));
  if (q.areaMin != null) params.set("areaMin", String(q.areaMin));
  if (q.areaMax != null) params.set("areaMax", String(q.areaMax));
  params.set("sort", q.sort ?? cfg["default_sort"] ?? "createdAt");
  params.set("sortAscending", cfg["sort_ascending"] ?? "false");
  params.set("page", String(Math.max(1, q.page)));
  params.set("per_page", String(q.perPage ?? numberOr(cfg["per_page"], DEFAULT_PER_PAGE)));

  return `${base}?${params.toString()}`;
}

/**
 * Fetch one Boligsiden search page with retry/backoff on 429/5xx. Best-effort:
 * returns `{ ok:false, listings:[] }` on any failure. Never throws.
 */
export async function fetchBoligsidenPage(
  db: D1Database,
  q: BoligsidenQuery,
): Promise<BoligsidenFetchResult> {
  const cfg = await getSourceConfig(db, "boligsiden");
  const ua = cfg["user_agent"] ?? DEFAULT_UA;
  const throttle = numberOr(cfg["rate_limit_ms"], 800);
  const url = buildUrl(cfg, q);

  await sleep(throttle);

  let lastError = "";
  for (let attempt = 0; attempt < 3; attempt++) {
    try {
      const res = await fetch(url, {
        headers: {
          "User-Agent": ua,
          Accept: "application/json",
        },
      });
      if (res.status === 429 || res.status >= 500) {
        lastError = `boligsiden HTTP ${res.status}`;
        await sleep(throttle * Math.pow(2, attempt + 1));
        continue;
      }
      if (!res.ok) {
        return { listings: [], found: 0, ok: false, error: `boligsiden HTTP ${res.status}` };
      }
      const body = (await res.json()) as Record<string, unknown>;
      const cases = extractCases(body);
      const found = extractTotal(body, cases.length);
      return {
        listings: cases.map(normalizeBoligsidenCard),
        found,
        ok: true,
      };
    } catch (err) {
      lastError = String(err);
      await sleep(throttle * Math.pow(2, attempt + 1));
    }
  }
  console.warn("boligsiden fetch failed", lastError);
  return { listings: [], found: 0, ok: false, error: lastError };
}

function extractCases(body: Record<string, unknown>): unknown[] {
  for (const key of ["cases", "results", "items"]) {
    const v = body[key];
    if (Array.isArray(v)) return v;
  }
  return [];
}

function extractTotal(body: Record<string, unknown>, fallback: number): number {
  for (const key of ["totalHits", "totalCount", "total", "count"]) {
    const v = body[key];
    if (typeof v === "number") return v;
  }
  return fallback;
}

/**
 * Map a Boligsiden `/search/cases` case object into a NormalizedListing. The
 * case + embedded `address.buildings[]` (BBR) schema drifts and fields may be
 * absent — every access is defensive and the function never throws.
 */
function normalizeBoligsidenCard(card: unknown): NormalizedListing {
  const c = (card ?? {}) as Record<string, unknown>;
  const address = asObject(c["address"]);
  const building = primaryBuilding(address);
  const municipality = asObject(address["municipality"]);
  const coords = asObject(c["coordinates"]) ?? asObject(address["coordinates"]);

  const id =
    firstString(c["caseID"], c["caseId"], c["id"], c["slug"]) ?? "";
  const url =
    firstString(c["caseUrl"], c["url"]) ??
    (firstString(c["slug"])
      ? `https://www.boligsiden.dk/adresse/${firstString(c["slug"])}`
      : "https://www.boligsiden.dk/");

  const addressTypeSlug = firstString(c["addressType"], get(c, "addressType.slug"));
  const description = firstString(c["description"], c["text"]);

  const priceDkk = toInt(firstString(c["priceCash"], c["price"], get(c, "priceCash.amount")));

  const heatingRaw = firstString(
    building?.["heatingInstallation"],
    building?.["supplementaryHeating"],
  );

  const row: NormalizedListing = {
    portal: "boligsiden",
    portal_listing_id: id,
    url,
    country: normalizeCountry(firstString(c["country"], "DK")) ?? "DK",

    property_type: mapAddressType(addressTypeSlug),
    holding_form: mapHoldingForm(addressTypeSlug),
    kiinteistotunnus: firstString(
      address["gstkvhx"],
      firstFromArray(address["bfeNumbers"]),
      address["bfeNumber"],
    ),
    address: composeAddress(address, c),
    municipality: firstString(
      address["cityName"],
      municipality?.["slug"],
      c["cityName"],
      municipality?.["name"],
    ),
    postal_code: firstString(address["zipCode"], c["zipCode"]),
    district: firstString(address["district"], c["district"]),
    lat: toNumber(firstString(coords?.["lat"], coords?.["latitude"], c["latitude"])),
    lon: toNumber(firstString(coords?.["lon"], coords?.["lng"], coords?.["longitude"], c["longitude"])),

    price_eur: dkkToEur(priceDkk),
    debt_free_price_eur: null,
    debt_share_eur: null,
    price_per_m2: dkkToEur(toNumber(firstString(c["perAreaPrice"], c["squaremeterPrice"]))),
    maintenance_charge_eur: dkkToEur(toNumber(firstString(c["monthlyExpense"], c["exp"]))),
    financing_charge_eur: null,
    ground_rent_eur_yr: null,

    living_area_m2: toNumber(
      firstString(c["housingArea"], c["weightedArea"], building?.["housingArea"]),
    ),
    total_area_m2: toNumber(firstString(c["weightedArea"], building?.["totalArea"], c["housingArea"])),
    plot_area_m2: toNumber(firstString(c["lotArea"], building?.["lotArea"])),
    room_count: toNumber(firstString(c["numberOfRooms"], c["rooms"])),
    room_layout: null,
    floors: toNumber(firstString(c["numberOfFloors"], building?.["numberOfFloors"])),

    year_built: toInt(firstString(c["yearBuilt"], building?.["yearBuilt"], building?.["buildYear"])),
    occupancy_year: null,
    roof_year: null,
    pipes_renovated_year: null,
    water_body: null,
    kiinteistovero_eur_yr: null,
    electricity_eur_yr: null,
    condition_class: mapCondition(building),
    inspection_status: null,
    frame_material: mapWallMaterial(building?.["externalWallMaterial"]),
    facade_material: firstString(building?.["externalWallMaterial"]),
    roof_material: mapRoofMaterial(building?.["roofingMaterial"]),
    energy_class: mapEnergyClass(c),
    e_value: null,
    risk_structures: extractRiskStructures(description, "DK"),

    plot_ownership: null,
    lease_end_year: null,
    shore: deriveShore(description),
    shore_sauna: null,

    heating_type: mapHeatingType(heatingRaw),
    heat_distribution: null,
    water_supply: null,
    sewer_system: null,
    broadband: null,
    sauna: null,
    parking: null,
    road_access: null,
    intended_use: null,
    zoning_status: null,
    description: description ?? null,

    status: mapStatus(c["status"]),
    raw_json: JSON.stringify(card),
  };
  return row;
}

/** DKK → EUR via the ERM-II peg; null in → null out, non-positive → null. */
function dkkToEur(dkk: number | null): number | null {
  if (dkk == null || dkk <= 0) return null;
  return Math.round(dkk / DKK_PER_EUR);
}

function primaryBuilding(address: Record<string, unknown>): Record<string, unknown> | undefined {
  const buildings = address["buildings"];
  if (!Array.isArray(buildings)) return undefined;
  for (const b of buildings) {
    const obj = asObject(b);
    if (obj) return obj;
  }
  return undefined;
}

function composeAddress(address: Record<string, unknown>, c: Record<string, unknown>): string | null {
  const road = firstString(address["road"], address["streetName"]);
  const houseNo = firstString(address["houseNumber"], address["houseNo"]);
  if (road) return houseNo ? `${road} ${houseNo}` : road;
  return firstString(address["addressText"], c["addressText"], c["address"]);
}

/** Boligsiden `addressTypes` slug → kontu `property_type` (pack §8). */
function mapAddressType(slug: unknown): string | null {
  const s = asciiFold(slug);
  if (s === "") return null;
  switch (s) {
    case "villa":
    case "villa apartment":
      return s === "villa" ? "detached" : "apartment";
    case "terraced house":
      return "terraced";
    case "condo":
      return "apartment";
    case "cooperative":
      return "apartment";
    case "holiday house":
      return "leisure";
    case "farm":
    case "hobby farm":
      return "farm";
    case "full year plot":
    case "holiday plot":
      return "plot";
    case "houseboat":
      return "other";
    default:
      return s;
  }
}

/** Co-op (`cooperative`) → andel; every other ownership address type → ejer. */
function mapHoldingForm(slug: unknown): string | null {
  const s = asciiFold(slug);
  if (s === "") return null;
  if (s === "cooperative") return "andel";
  if (s === "full year plot" || s === "holiday plot" || s === "houseboat") return null;
  return "ejer";
}

/** BBR `heatingInstallation` (pack §8) → kontu `heating_type`. */
function mapHeatingType(raw: unknown): string | null {
  const s = asciiFold(raw);
  if (s === "") return null;
  if (/fjernvarme|blokvarme/.test(s)) return "district";
  if (/varmepumpe/.test(s)) return "heat_pump";
  if (/centralvarme/.test(s)) return "central_boiler";
  if (/elvarme|elektrisk/.test(s)) return "direct_electric";
  if (/ovn til fast|fast og flydende|braendsel|braendsler/.test(s)) return "stove_solid_liquid";
  if (/ingen varmeinstallation|ingen varme/.test(s)) return "none";
  return s;
}

/** BBR `roofingMaterial` (pack §8) → kontu `roof_material` token + asbestos flag. */
function mapRoofMaterial(raw: unknown): string | null {
  const s = asciiFold(raw);
  if (s === "") return null;
  if (/fibercement.*asbest|asbest/.test(s) && !/uden asbest/.test(s)) return "fibercement_asbestos";
  if (/fibercement/.test(s)) return "fibercement";
  if (/betontagsten|beton/.test(s)) return "concrete_tile";
  if (/tegl/.test(s)) return "clay_tile";
  if (/tagpap/.test(s)) return "felt";
  if (/straatag|straa/.test(s)) return "thatch";
  if (/metal/.test(s)) return "metal";
  if (/plast|glas|levende/.test(s)) return "other";
  return s;
}

/** BBR `externalWallMaterial` (pack §8) → kontu frame token. */
function mapWallMaterial(raw: unknown): string | null {
  const s = asciiFold(raw);
  if (s === "") return null;
  if (/fibercement.*asbest|asbest/.test(s) && !/uden asbest/.test(s)) return "asbestos";
  if (/mursten/.test(s)) return "brick";
  if (/bindingsvaerk/.test(s)) return "half_timber";
  if (/letbetonsten|gasbeton|porebeton/.test(s)) return "aerated_concrete";
  if (/betonelementer|beton/.test(s)) return "concrete_panel";
  if (/trae/.test(s)) return "wood";
  if (/glas|ingen/.test(s)) return "other";
  return s;
}

/**
 * Condition from the worst BBR room-condition grade (kitchen/bathroom/toilet).
 * Boligsiden cases carry no tilstandsrapport red/yellow grade; the room
 * conditions are the only structured condition signal on the listing.
 */
function mapCondition(building: Record<string, unknown> | undefined): string | null {
  if (!building) return null;
  const grades = [
    building["kitchenCondition"],
    building["bathroomCondition"],
    building["toiletCondition"],
  ];
  let worst: string | null = null;
  let worstRank = -1;
  for (const g of grades) {
    const cls = roomConditionClass(g);
    if (cls == null) continue;
    const rank = CONDITION_RANK[cls] ?? -1;
    if (rank > worstRank) {
      worstRank = rank;
      worst = cls;
    }
  }
  return worst;
}

const CONDITION_RANK: Record<string, number> = {
  minor: 0,
  serious: 1,
  critical: 2,
};

function roomConditionClass(raw: unknown): string | null {
  const s = asciiFold(raw);
  if (s === "") return null;
  if (/roed|red|kritisk|kritis/.test(s)) return "critical";
  if (/gul|yellow|alvorlig|serious/.test(s)) return "serious";
  if (/graa|grey|gray|mindre|minor|kosmet/.test(s)) return "minor";
  return null;
}

/**
 * Energy label → A–G. Boligsiden carries `energyLabel.classification` like
 * `a2020`/`a2015`/`b`; the shared `normalizeEnergyClass` matches an isolated
 * `[A-G]` and would miss `a2020` (letter glued to digits), so collapse the
 * `a20xx` form here first.
 */
function mapEnergyClass(c: Record<string, unknown>): string | null {
  const raw = firstString(
    get(c, "energyLabel.classification"),
    c["energyLabel"],
    c["energyClass"],
    get(c, "energyLabel.label"),
  );
  if (raw == null) return null;
  const collapsed = raw.trim().replace(/^([a-gA-G])\s*20\d{2}$/, "$1");
  return normalizeEnergyClass(collapsed);
}

/** Text-only shore boost (no native filter): Danish waterfront keywords. */
function deriveShore(description: unknown): string | null {
  const s = asciiFold(description);
  if (s === "") return null;
  if (/havudsigt|soeudsigt|soudsigt|vandnaer|ved vandet|strand|strandgrund/.test(s)) {
    return "rantaoikeus";
  }
  return null;
}

function mapStatus(raw: unknown): string {
  const s = asciiFold(raw);
  if (/solgt|sold/.test(s)) return "sold";
  if (/reserveret|reserved/.test(s)) return "reserved";
  if (/(trukket|withdrawn|removed|inaktiv|inactive|expired|udloeb)/.test(s)) return "withdrawn";
  return "active";
}

function asObject(v: unknown): Record<string, unknown> {
  return v != null && typeof v === "object" && !Array.isArray(v)
    ? (v as Record<string, unknown>)
    : {};
}

function firstFromArray(v: unknown): unknown {
  return Array.isArray(v) && v.length > 0 ? v[0] : undefined;
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

function get(obj: unknown, path: string): unknown {
  if (obj == null || typeof obj !== "object") return undefined;
  let cur: unknown = obj;
  for (const key of path.split(".")) {
    if (cur == null || typeof cur !== "object") return undefined;
    cur = (cur as Record<string, unknown>)[key];
  }
  return cur;
}

function toNumber(v: unknown): number | null {
  if (v == null) return null;
  if (typeof v === "number") return Number.isFinite(v) ? v : null;
  if (typeof v === "boolean") return null;
  let s = String(v).trim();
  if (s === "") return null;
  s = s.replace(/[^\d,.\-]/g, "");
  if (s === "" || s === "-") return null;
  const hasComma = s.includes(",");
  const hasDot = s.includes(".");
  if (hasComma && hasDot) {
    s = s.replace(/\./g, "").replace(",", ".");
  } else if (hasComma) {
    s = s.replace(",", ".");
  }
  const n = Number(s);
  return Number.isFinite(n) ? n : null;
}

function toInt(v: unknown): number | null {
  const n = toNumber(v);
  return n == null ? null : Math.round(n);
}

function numberOr(s: string | undefined, def: number): number {
  const n = s == null ? NaN : Number(s);
  return Number.isFinite(n) ? n : def;
}
