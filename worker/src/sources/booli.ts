/**
 * Plane-A Booli (SE) adapter. Self-contained: maps the Swedish portal vocabulary
 * onto kontu's normalized enums (per the SE pack §8) and the EUR cost model, then
 * emits NormalizedListing[]. Booli is the ToS-cleanest SE path — its robots.txt
 * leaves `/sok`, `/graphql` and `/api` un-disallowed — so it is the primary SE
 * source. Disposable by design (FI parity): every failure (429, Cloudflare block,
 * datacenter-IP gate, schema drift) returns `{ ok:false, listings:[] }` instead of
 * throwing, so a bad page can never break the crawl. All volatile params/endpoints
 * come from `source_config` in D1; nothing portal-specific is hardcoded that a
 * residential-IP recon couldn't override.
 */
import { parse } from "node-html-parser";
import { getSourceConfig } from "../db";
import type { NormalizedListing } from "../normalize";

/** Build constant from the SE pack: store SEK as source-of-truth, convert at 11.3 SEK/EUR. */
const SEK_PER_EUR = 11.3;

const DEFAULT_UA =
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

const sleep = (ms: number): Promise<void> => new Promise((r) => setTimeout(r, ms));

export interface BooliQuery {
  /** Booli internal area IDs (NOT SCB kommun codes) — e.g. Stockholm kommun=1. Repeatable. */
  areaIds?: (string | number)[];
  /** Free-text location name; used by the GraphQL `location` filter when no areaIds. */
  location?: string;
  priceMin?: number;
  priceMax?: number;
  roomsMin?: number;
  roomsMax?: number;
  areaMin?: number;
  areaMax?: number;
  /** kontu property_type tokens (`detached_house`, `leisure_home`, …) or raw Booli objectType values. */
  propertyTypes?: string[];
  /** 1-based page. */
  page: number;
  limit?: number;
}

export interface BooliFetchResult {
  listings: NormalizedListing[];
  found: number;
  ok: boolean;
  error?: string;
}

/**
 * Fetch ONE page of Booli listings. Tries the documented GraphQL endpoint first
 * (`api.booli.se/graphql`), then falls back to scraping the public `/sok` search
 * page's embedded `__NEXT_DATA__` JSON — the more robust path when the GraphQL
 * persisted-query hash is unknown. Best-effort: returns `{ ok:false }` on any
 * failure and never throws.
 */
export async function fetchBooliPage(db: D1Database, q: BooliQuery): Promise<BooliFetchResult> {
  const cfg = await getSourceConfig(db, "booli");
  const throttle = numberOr(cfg["rate_limit_ms"], 1800);
  await sleep(throttle);

  const useGraphql = (cfg["use_graphql"] ?? "false").toLowerCase() === "true";
  if (useGraphql) {
    const gql = await tryGraphql(cfg, q, throttle);
    if (gql.ok) return gql;
    console.warn("booli graphql path failed, falling back to __NEXT_DATA__", gql.error);
  }
  return tryNextData(cfg, q, throttle);
}

/**
 * GraphQL path. Off by default (`use_graphql`) because the persisted-query SHA-256
 * hash that `api.booli.se/graphql` requires is NOT verified — naive operations are
 * rejected. When a recon supplies `graphql_query` + `graphql_operation` (or a
 * persisted-hash) via source_config, this replays them with the documented filter
 * inputs: `location`, `minPrice`/`maxPrice`, `minRooms`/`maxRooms`, `minArea`/`maxArea`,
 * `objectType`.
 */
async function tryGraphql(
  cfg: Record<string, string>,
  q: BooliQuery,
  throttle: number,
): Promise<BooliFetchResult> {
  const endpoint = cfg["graphql_url"] ?? "https://api.booli.se/graphql";
  const ua = cfg["user_agent"] ?? DEFAULT_UA;
  const query = cfg["graphql_query"];
  if (!query) {
    return { listings: [], found: 0, ok: false, error: "no graphql_query in source_config" };
  }
  const variables: Record<string, unknown> = {
    location: q.location ?? null,
    areaId: q.areaIds && q.areaIds.length ? String(q.areaIds[0]) : null,
    minPrice: q.priceMin ?? null,
    maxPrice: q.priceMax ?? null,
    minRooms: q.roomsMin ?? null,
    maxRooms: q.roomsMax ?? null,
    minArea: q.areaMin ?? null,
    maxArea: q.areaMax ?? null,
    objectType: mapObjectTypes(cfg, q.propertyTypes),
    page: q.page,
    limit: q.limit ?? 35,
  };
  const operationName = cfg["graphql_operation"] || undefined;
  const body = JSON.stringify({ operationName, query, variables });

  let lastError = "";
  for (let attempt = 0; attempt < 3; attempt++) {
    try {
      const res = await fetch(endpoint, {
        method: "POST",
        headers: {
          "User-Agent": ua,
          "Content-Type": "application/json",
          Accept: "application/json",
          "Accept-Language": "sv-SE,sv;q=0.9",
          Origin: "https://www.booli.se",
          Referer: "https://www.booli.se/sok/till-salu",
        },
        body,
      });
      if (res.status === 429 || res.status >= 500) {
        lastError = `booli graphql HTTP ${res.status}`;
        await sleep(throttle * Math.pow(2, attempt + 1));
        continue;
      }
      if (!res.ok) {
        return { listings: [], found: 0, ok: false, error: `booli graphql HTTP ${res.status}` };
      }
      const parsed = (await res.json()) as Record<string, unknown>;
      const result = extractGraphqlResult(parsed);
      return {
        listings: result.cards.map(normalizeBooliCard),
        found: result.found,
        ok: true,
      };
    } catch (err) {
      lastError = String(err);
      await sleep(throttle * Math.pow(2, attempt + 1));
    }
  }
  return { listings: [], found: 0, ok: false, error: lastError || "booli graphql exhausted retries" };
}

/**
 * Robust fallback: GET the public Booli search page and parse the Next.js
 * `<script id="__NEXT_DATA__">` JSON blob (the SSR data Booli embeds for hydration).
 * Tolerant of 429/5xx with exponential backoff; never throws.
 */
async function tryNextData(
  cfg: Record<string, string>,
  q: BooliQuery,
  throttle: number,
): Promise<BooliFetchResult> {
  const url = buildSearchUrl(cfg, q);
  const ua = cfg["user_agent"] ?? DEFAULT_UA;

  let lastError = "";
  for (let attempt = 0; attempt < 3; attempt++) {
    try {
      const res = await fetch(url, {
        headers: {
          "User-Agent": ua,
          Accept: "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
          "Accept-Language": "sv-SE,sv;q=0.9,en;q=0.8",
          Referer: "https://www.booli.se/",
        },
      });
      if (res.status === 429 || res.status >= 500) {
        lastError = `booli search HTTP ${res.status}`;
        await sleep(throttle * Math.pow(2, attempt + 1));
        continue;
      }
      if (!res.ok) {
        return { listings: [], found: 0, ok: false, error: `booli search HTTP ${res.status}` };
      }
      const html = await res.text();
      const data = extractNextData(html);
      if (!data) {
        return { listings: [], found: 0, ok: false, error: "booli __NEXT_DATA__ not found (block or schema drift)" };
      }
      const { cards, found } = extractSearchResult(data);
      return {
        listings: cards.map(normalizeBooliCard),
        found,
        ok: true,
      };
    } catch (err) {
      lastError = String(err);
      await sleep(throttle * Math.pow(2, attempt + 1));
    }
  }
  console.warn("booli fetch failed", lastError);
  return { listings: [], found: 0, ok: false, error: lastError || "booli search exhausted retries" };
}

/** `https://www.booli.se/sok/till-salu?areaIds=…&objectType=…&page=…` per the SE pack §6B grammar. */
function buildSearchUrl(cfg: Record<string, string>, q: BooliQuery): string {
  const base = cfg["search_url"] ?? "https://www.booli.se/sok/till-salu";
  const params = new URLSearchParams();
  for (const id of q.areaIds ?? []) params.append("areaIds", String(id));
  if ((q.areaIds == null || q.areaIds.length === 0) && q.location) params.set("location", q.location);
  const objectTypes = mapObjectTypes(cfg, q.propertyTypes);
  if (objectTypes.length) params.set("objectType", objectTypes.join(","));
  if (q.priceMin != null) params.set("minPrice", String(q.priceMin));
  if (q.priceMax != null) params.set("maxPrice", String(q.priceMax));
  if (q.roomsMin != null) params.set("minRooms", String(q.roomsMin));
  if (q.roomsMax != null) params.set("maxRooms", String(q.roomsMax));
  if (q.areaMin != null) params.set("minLivingArea", String(q.areaMin));
  if (q.areaMax != null) params.set("maxLivingArea", String(q.areaMax));
  params.set("page", String(q.page));
  const qs = params.toString();
  return qs ? `${base}?${qs}` : base;
}

/**
 * Map kontu property_type tokens → Booli `objectType` values (SE pack §8A). The
 * `kedjehus/parhus/radhus` group collapses to one combined Booli value. A
 * source_config `object_type_map` JSON override wins; otherwise the built-in table.
 * Unknown tokens pass through verbatim (caller may already hold raw Booli values).
 */
function mapObjectTypes(cfg: Record<string, string>, tokens: string[] | undefined): string[] {
  if (!tokens || tokens.length === 0) return [];
  const override = parseJsonObject(cfg["object_type_map"]);
  const map: Record<string, string> = override
    ? Object.fromEntries(Object.entries(override).map(([k, v]) => [k, String(v)]))
    : {
        detached_house: "Villa",
        villa: "Villa",
        semi_detached: "Kedjehus-Parhus-Radhus",
        terraced: "Kedjehus-Parhus-Radhus",
        apartment: "Lägenhet",
        leisure_home: "Fritidshus",
        farm: "Gård",
        plot: "Tomt/Mark",
      };
  const out: string[] = [];
  for (const t of tokens) {
    const v = map[t] ?? map[asciiLower(t)] ?? t;
    if (!out.includes(v)) out.push(v);
  }
  return out;
}

function extractNextData(html: string): unknown {
  try {
    const root = parse(html);
    const el = root.querySelector("script#__NEXT_DATA__");
    const raw = el?.text ?? el?.innerText;
    if (raw && raw.trim() !== "") return JSON.parse(raw);
  } catch (err) {
    console.warn("booli __NEXT_DATA__ parse failed", String(err));
  }
  return null;
}

/**
 * Walk the (drift-prone) Next.js data tree for the listing array + total. Booli's
 * SSR shape is not pinned, so probe a broad set of candidate paths and, as a last
 * resort, deep-scan for the first array of listing-shaped objects.
 */
function extractSearchResult(data: unknown): { cards: Record<string, unknown>[]; found: number } {
  const root = (data ?? {}) as Record<string, unknown>;
  const pageProps = (getPath(root, "props.pageProps") ?? {}) as Record<string, unknown>;
  const containers = [
    getPath(pageProps, "searchResult"),
    getPath(pageProps, "search"),
    getPath(pageProps, "listings"),
    getPath(pageProps, "data.search"),
    getPath(pageProps, "initialData.search"),
    pageProps,
  ];
  for (const cont of containers) {
    if (cont == null || typeof cont !== "object") continue;
    const c = cont as Record<string, unknown>;
    const cards = firstArray(c["result"], c["results"], c["listings"], c["items"], c["objects"], c["edges"]);
    if (cards.length) {
      return { cards: cards.map(unwrapEdge), found: extractTotal(c, cards.length) };
    }
  }
  const deep = deepFindListingArray(root);
  return { cards: deep, found: deep.length };
}

/** Pull the listing array + total out of a GraphQL `data.search`-style envelope. */
function extractGraphqlResult(parsed: Record<string, unknown>): {
  cards: Record<string, unknown>[];
  found: number;
} {
  const data = (parsed["data"] ?? parsed) as Record<string, unknown>;
  const containers = [
    getPath(data, "search"),
    getPath(data, "listings"),
    getPath(data, "searchForSale"),
    getPath(data, "soldProperties"),
    data,
  ];
  for (const cont of containers) {
    if (cont == null || typeof cont !== "object") continue;
    const c = cont as Record<string, unknown>;
    const cards = firstArray(c["result"], c["results"], c["listings"], c["items"], c["objects"], c["edges"]);
    if (cards.length) {
      return { cards: cards.map(unwrapEdge), found: extractTotal(c, cards.length) };
    }
  }
  const deep = deepFindListingArray(data);
  return { cards: deep, found: deep.length };
}

function unwrapEdge(v: unknown): Record<string, unknown> {
  if (v && typeof v === "object") {
    const node = (v as Record<string, unknown>)["node"];
    if (node && typeof node === "object") return node as Record<string, unknown>;
    return v as Record<string, unknown>;
  }
  return {};
}

function looksLikeListing(v: unknown): boolean {
  if (v == null || typeof v !== "object") return false;
  const o = v as Record<string, unknown>;
  const node = (o["node"] && typeof o["node"] === "object" ? o["node"] : o) as Record<string, unknown>;
  const hasId = node["booliId"] != null || node["id"] != null || node["objectId"] != null;
  const hasShape =
    "listPrice" in node ||
    "price" in node ||
    "rooms" in node ||
    "livingArea" in node ||
    "objectType" in node ||
    "location" in node ||
    "streetAddress" in node;
  return hasId && hasShape;
}

/** BFS for the first array whose elements look like listing cards. Bounded depth. */
function deepFindListingArray(root: unknown): Record<string, unknown>[] {
  const queue: Array<{ v: unknown; depth: number }> = [{ v: root, depth: 0 }];
  let visited = 0;
  while (queue.length && visited < 5000) {
    const { v, depth } = queue.shift() as { v: unknown; depth: number };
    visited++;
    if (depth > 8 || v == null || typeof v !== "object") continue;
    if (Array.isArray(v)) {
      if (v.length && v.some(looksLikeListing)) {
        return v.filter(looksLikeListing).map(unwrapEdge);
      }
      for (const item of v) queue.push({ v: item, depth: depth + 1 });
      continue;
    }
    for (const val of Object.values(v as Record<string, unknown>)) {
      queue.push({ v: val, depth: depth + 1 });
    }
  }
  return [];
}

function extractTotal(c: Record<string, unknown>, fallback: number): number {
  for (const key of ["totalCount", "total", "totalResults", "count", "hits", "numberOfHits"]) {
    const v = c[key];
    if (typeof v === "number" && Number.isFinite(v)) return v;
    const n = toNumber(v);
    if (n != null) return Math.round(n);
  }
  const page = getPath(c, "pageInfo.totalCount");
  const n = toNumber(page);
  return n != null ? Math.round(n) : fallback;
}

/**
 * Map one Booli listing card → NormalizedListing. `country` is always "SE";
 * SEK prices/fees are converted to EUR at 11.3 SEK/EUR; every field is read
 * defensively (the card schema drifts between the GraphQL and __NEXT_DATA__ paths)
 * and is null when the source carries no value. The full card is retained as
 * `raw_json`.
 */
function normalizeBooliCard(card: unknown): NormalizedListing {
  const c = (card ?? {}) as Record<string, unknown>;

  const id =
    firstString(c["booliId"], c["id"], c["objectId"], getPath(c, "listing.id")) ?? "";
  const slug = firstString(c["slug"], getPath(c, "url.slug"));
  const url =
    firstString(c["url"], getPath(c, "url.href"), c["permalink"]) ??
    (slug
      ? `https://www.booli.se/annons/${slug}`
      : id
        ? `https://www.booli.se/annons/${id}`
        : "https://www.booli.se/");

  const street = firstString(
    getPath(c, "streetAddress"),
    getPath(c, "address.streetAddress"),
    getPath(c, "location.address.streetAddress"),
    getPath(c, "location.streetAddress"),
    c["address"],
  );
  const municipality = firstString(
    getPath(c, "location.region.municipalityName"),
    getPath(c, "location.municipalityName"),
    getPath(c, "location.namedAreas"),
    getPath(c, "municipality"),
    getPath(c, "location.region.name"),
  );
  const district = firstString(
    getPath(c, "location.namedAreas.0"),
    getPath(c, "location.district"),
    getPath(c, "districtName"),
  );

  const objectTypeRaw = firstString(
    c["objectType"],
    getPath(c, "propertyType"),
    getPath(c, "estateType"),
  );
  const tenureRaw = firstString(
    c["tenureForm"],
    c["upplatelseform"],
    c["ownershipType"],
    getPath(c, "tenure"),
  );

  const descriptionParts = [
    firstString(c["descriptiveAreaName"]),
    firstString(c["description"], c["body"], getPath(c, "listing.description")),
  ].filter((s): s is string => s != null);
  const description = descriptionParts.length ? descriptionParts.join(" — ") : null;
  const shoreText = [
    description ?? "",
    objectTypeRaw ?? "",
    firstString(c["water"], c["waterDistance"]) ?? "",
  ].join(" ");

  const listPriceSek = toNumber(
    firstString(getPath(c, "listPrice.value"), getPath(c, "listPrice"), c["price"], c["amount"]),
  );
  const rentSek = toNumber(firstString(getPath(c, "rent.value"), getPath(c, "rent"), c["monthlyFee"]));
  const operatingCostSek = toNumber(
    firstString(getPath(c, "operatingCost.value"), getPath(c, "operatingCost"), c["runningCosts"]),
  );
  const sqmPriceSek = toNumber(
    firstString(getPath(c, "squareMeterPrice.value"), getPath(c, "squareMeterPrice"), c["pricePerM2"]),
  );

  const lat = toNumber(
    firstString(getPath(c, "latitude"), getPath(c, "location.position.latitude"), getPath(c, "location.latitude")),
  );
  const lon = toNumber(
    firstString(getPath(c, "longitude"), getPath(c, "location.position.longitude"), getPath(c, "location.longitude")),
  );

  const row: NormalizedListing = {
    portal: "booli",
    portal_listing_id: id,
    url,
    country: "SE",
    property_type: mapPropertyType(objectTypeRaw),
    holding_form: mapHoldingForm(tenureRaw ?? objectTypeRaw),
    kiinteistotunnus: firstString(c["propertyDesignation"], c["fastighetsbeteckning"]),
    address: street,
    municipality,
    postal_code: firstString(getPath(c, "location.postalCode"), c["postalCode"], c["zipCode"]),
    district,
    lat,
    lon,
    price_eur: sekToEur(listPriceSek),
    debt_free_price_eur: null,
    debt_share_eur: null,
    price_per_m2: sekToEur(sqmPriceSek),
    maintenance_charge_eur: sekToEur(rentSek),
    financing_charge_eur: null,
    ground_rent_eur_yr: sekToEur(toNumber(firstString(c["groundRent"], c["tomtrattsavgald"]))),
    living_area_m2: toNumber(
      firstString(getPath(c, "livingArea.value"), c["livingArea"], c["boarea"], c["area"]),
    ),
    total_area_m2: toNumber(firstString(getPath(c, "additionalArea.value"), c["additionalArea"], c["biarea"])),
    plot_area_m2: toNumber(firstString(getPath(c, "plotArea.value"), c["plotArea"], c["tomtarea"], c["lotArea"])),
    room_count: toNumber(firstString(getPath(c, "rooms.value"), c["rooms"], c["roomCount"])),
    room_layout: firstString(c["roomLayout"]),
    floors: toNumber(firstString(c["floor"], c["floors"], getPath(c, "floor.value"))),
    year_built: toInt(firstString(c["constructionYear"], c["byggar"], c["buildYear"], c["yearBuilt"])),
    occupancy_year: null,
    roof_year: null,
    pipes_renovated_year: null,
    water_body: null,
    kiinteistovero_eur_yr: null,
    electricity_eur_yr: null,
    condition_class: null,
    inspection_status: null,
    frame_material: null,
    facade_material: null,
    roof_material: firstString(c["roofMaterial"]),
    energy_class: mapEnergyClass(firstString(getPath(c, "energyClass.classification"), c["energyClass"], c["energyClassification"])),
    e_value: toNumber(firstString(getPath(c, "energyClass.performance"), c["energyPerformance"])),
    risk_structures: [],
    plot_ownership: mapPlotOwnership(tenureRaw),
    lease_end_year: null,
    shore: mapShore(shoreText),
    shore_sauna: null,
    heating_type: mapHeatingType(firstString(c["heating"], c["uppvarmning"], c["heatingType"])),
    heat_distribution: null,
    water_supply: mapWaterSupply(firstString(c["water"], c["vatten"], c["waterSupply"])),
    sewer_system: mapSewer(firstString(c["sewer"], c["avlopp"], c["sewerSystem"])),
    broadband: firstString(c["broadband"], c["bredband"]),
    sauna: null,
    parking: firstString(c["parking"], c["parkering"]),
    road_access: mapRoadAccess(firstString(c["road"], c["vag"], c["roadAccess"])),
    intended_use: null,
    zoning_status: null,
    description,
    status: mapBooliStatus(firstString(c["status"], c["listingStatus"], c["state"])),
    raw_json: safeStringify(card),
  };
  applyOperatingCost(row, operatingCostSek);
  return row;
}

/** Fold a combined Booli `driftkostnad` (SEK/yr) into a monthly EUR maintenance estimate when no explicit fee exists. */
function applyOperatingCost(row: NormalizedListing, operatingCostSek: number | null): void {
  if (row.maintenance_charge_eur != null || operatingCostSek == null || operatingCostSek <= 0) return;
  row.maintenance_charge_eur = sekToEur(Math.round(operatingCostSek / 12));
}

/** SEK → EUR at the SE-pack build constant (11.3), rounded. Non-positive / missing → null. */
function sekToEur(sek: number | null): number | null {
  if (sek == null || !Number.isFinite(sek) || sek <= 0) return null;
  return Math.round(sek / SEK_PER_EUR);
}

const PROPERTY_TYPE_TABLE: Array<[RegExp, string]> = [
  [/\bvilla\b|enbostadshus|fristaende|friliggande/, "detached_house"],
  [/kedjehus|parhus/, "semi_detached"],
  [/radhus/, "terraced"],
  [/fritidshus|fritid|stuga/, "leisure_home"],
  [/\bgard\b|gard\/skog|lantbruk|jordbruk/, "farm"],
  [/\btomt\b|tomt\/mark|mark\b|markomrade/, "plot"],
  [/lagenhet|bostadsratt|\bbr\b/, "apartment"],
];

/** Booli `objectType` / free-text → kontu property_type (SE pack §8A). */
function mapPropertyType(raw: unknown): string | null {
  const s = asciiLower(raw);
  if (s === "") return null;
  for (const [re, val] of PROPERTY_TYPE_TABLE) {
    if (re.test(s)) return val;
  }
  return null;
}

/** Swedish tenure / object type → kontu holding_form (SE pack §8B). */
function mapHoldingForm(raw: unknown): string | null {
  const s = asciiLower(raw);
  if (s === "") return null;
  if (/tomtratt/.test(s)) return "leasehold_land";
  if (/bostadsratt|\bbr\b|lagenhet/.test(s)) return "coop";
  if (/hyresratt|hyra\b/.test(s)) return "rental";
  if (/aganderatt|agande|fastighet|friliggande|villa|radhus|kedjehus|parhus|gard|tomt/.test(s)) {
    return "freehold";
  }
  return null;
}

/** Swedish uppvärmning → kontu heating_type (SE pack §8C). */
function mapHeatingType(raw: unknown): string | null {
  const s = asciiLower(raw);
  if (s === "") return null;
  if (/fjarrvarme/.test(s)) return "district";
  if (/bergvarme|jordvarme|berg-?\/?jord|vattenvarmepump|vavp\b/.test(s)) return "ground_source";
  if (/luftvarmepump|luft-?luft|luftvarme/.test(s)) return "air_source_heatpump";
  if (/vattenburen el/.test(s)) return "electric_hydronic";
  if (/direktverkande|direktel|elvarme|elradiator/.test(s)) return "direct_electric";
  if (/pellet|\bved\b|biobransle|braslevarme|kamin/.test(s)) return "wood_biomass";
  if (/\bolja\b|oljepanna/.test(s)) return "oil";
  if (/\bel\b|elvarme/.test(s)) return "direct_electric";
  return null;
}

/** Swedish shore lexicon → kontu shore (SE pack §8D). Free-text only; geo cross-check is the robust signal. */
function mapShore(raw: unknown): string | null {
  const s = asciiLower(raw);
  if (s === "") return null;
  if (/egen strand|sjotomt|strandtomt|egen brygga|sjonara tomt|vid vattnet|first row|forsta parkett/.test(s)) {
    return "own_shore";
  }
  if (/strandratt|gemensam strand|delad strand|sjoratt|gemensam brygga/.test(s)) return "shore_right";
  if (/sjoutsikt|havsutsikt|vattenutsikt|sjoutblick/.test(s)) return "water_view";
  return null;
}

/** Swedish upplåtelseform (land) → kontu plot_ownership. `tomträtt` = leased land. */
function mapPlotOwnership(raw: unknown): string | null {
  const s = asciiLower(raw);
  if (s === "") return null;
  if (/tomtratt|arrende|ofri grund/.test(s)) return "vuokra";
  if (/aganderatt|agande|frikopt|egen tomt|fastighet/.test(s)) return "oma";
  return null;
}

/** kommunalt VA vs egen brunn (SE pack §8F). */
function mapWaterSupply(raw: unknown): string | null {
  const s = asciiLower(raw);
  if (s === "") return null;
  if (/kommunal/.test(s)) return "municipal";
  if (/egen brunn|borrad brunn|gravd brunn|enskild brunn|\bbrunn\b/.test(s)) return "well";
  return null;
}

/** kommunalt avlopp vs enskilt avlopp variants (SE pack §8F / §4). */
function mapSewer(raw: unknown): string | null {
  const s = asciiLower(raw);
  if (s === "") return null;
  if (/kommunal/.test(s)) return "municipal";
  if (/minireningsverk/.test(s)) return "minireningsverk";
  if (/markbadd/.test(s)) return "markbadd";
  if (/infiltration/.test(s)) return "infiltration";
  if (/sluten tank|trekammarbrunn|enskilt avlopp|enskild/.test(s)) return "infiltration";
  return null;
}

/** kommunal väg vs enskild väg / samfällighet (SE pack §8F) → drives the private-road cost line. */
function mapRoadAccess(raw: unknown): string | null {
  const s = asciiLower(raw);
  if (s === "") return null;
  if (/kommunal/.test(s)) return "public";
  if (/enskild|samfallighet|samfalld/.test(s)) return "private";
  return null;
}

/** Boverket energy class A–G (plus the 2026 top class A0) → uppercased ordinal letter. */
function mapEnergyClass(raw: unknown): string | null {
  const s = String(raw ?? "").trim().toUpperCase();
  if (s === "") return null;
  if (/\bA0\b/.test(s)) return "A0";
  const m = s.match(/\b([A-G])\b/);
  return m ? (m[1] ?? null) : null;
}

function mapBooliStatus(raw: unknown): string {
  const s = asciiLower(raw);
  if (s === "") return "active";
  if (/sald|sald|sold|slutpris/.test(s)) return "sold";
  if (/reserverad|bokad|reserved/.test(s)) return "reserved";
  if (/borttagen|avpublicerad|removed|withdrawn|expired/.test(s)) return "withdrawn";
  return "active";
}

// ---- self-contained local helpers (do NOT import non-exported normalize internals) ----

/** Lowercase, strip Swedish/Nordic diacritics (å/ä/ö → a/a/o), collapse whitespace. */
function asciiLower(input: unknown): string {
  const s = typeof input === "string" ? input : input == null ? "" : String(input);
  return s
    .normalize("NFKD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/\s+/g, " ")
    .trim();
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

/** Best-effort number coercion accepting `"3 500 000 kr"`, `"1 234,5"`, numbers. */
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

/** Dotted-path getter; numeric segments index into arrays. Never throws. */
function getPath(obj: unknown, path: string): unknown {
  if (obj == null || typeof obj !== "object") return undefined;
  let cur: unknown = obj;
  for (const key of path.split(".")) {
    if (cur == null || typeof cur !== "object") return undefined;
    cur = (cur as Record<string, unknown>)[key];
  }
  return cur;
}

function firstArray(...vals: unknown[]): unknown[] {
  for (const v of vals) {
    if (Array.isArray(v)) return v;
  }
  return [];
}

function numberOr(s: string | undefined, def: number): number {
  const n = s == null ? NaN : Number(s);
  return Number.isFinite(n) ? n : def;
}

function parseJsonObject(s: string | undefined): Record<string, unknown> | null {
  if (!s) return null;
  try {
    const v = JSON.parse(s);
    return v && typeof v === "object" && !Array.isArray(v) ? (v as Record<string, unknown>) : null;
  } catch {
    return null;
  }
}

function safeStringify(v: unknown): string {
  try {
    return JSON.stringify(v ?? {});
  } catch {
    return "{}";
  }
}
