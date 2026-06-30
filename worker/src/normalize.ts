/**
 * Pure normalization layer. No network, no D1 — every function here must be
 * deterministic and total (never throw on missing/garbage source data) so it can
 * be exhaustively unit-tested and reused from the crawler and from tests alike.
 */

export interface NormalizedListing {
  portal: string;
  portal_listing_id: string;
  url: string;
  /** ISO country code of the market (FI/SE/NO/DK/IS). */
  country: string | null;

  property_type: string | null;
  holding_form: string | null;
  kiinteistotunnus: string | null;
  address: string | null;
  municipality: string | null;
  postal_code: string | null;
  district: string | null;
  lat: number | null;
  lon: number | null;

  price_eur: number | null;
  debt_free_price_eur: number | null;
  debt_share_eur: number | null;
  price_per_m2: number | null;
  maintenance_charge_eur: number | null;
  financing_charge_eur: number | null;
  ground_rent_eur_yr: number | null;

  living_area_m2: number | null;
  total_area_m2: number | null;
  plot_area_m2: number | null;
  room_count: number | null;
  room_layout: string | null;
  floors: number | null;

  year_built: number | null;
  occupancy_year: number | null;
  roof_year: number | null;
  pipes_renovated_year: number | null;
  water_body: string | null;
  kiinteistovero_eur_yr: number | null;
  electricity_eur_yr: number | null;
  condition_class: string | null;
  inspection_status: string | null;
  frame_material: string | null;
  facade_material: string | null;
  roof_material: string | null;
  energy_class: string | null;
  e_value: number | null;
  risk_structures: string[];

  plot_ownership: string | null;
  lease_end_year: number | null;
  shore: string | null;
  shore_sauna: number | null;

  heating_type: string | null;
  heat_distribution: string | null;
  water_supply: string | null;
  sewer_system: string | null;
  broadband: string | null;
  sauna: string | null;
  parking: string | null;
  road_access: string | null;
  intended_use: string | null;
  zoning_status: string | null;
  description: string | null;

  status: string;
  raw_json: string;
}

const DIACRITICS: Record<string, string> = {
  ä: "a",
  ö: "o",
  å: "a",
  Ä: "a",
  Ö: "o",
  Å: "a",
  é: "e",
  è: "e",
  ü: "u",
};

/** Lowercase, strip Finnish/Nordic diacritics, collapse whitespace. */
export function asciiFold(input: unknown): string {
  const s = typeof input === "string" ? input : input == null ? "" : String(input);
  let out = "";
  for (const ch of s) {
    out += DIACRITICS[ch] ?? ch;
  }
  return out
    .normalize("NFKD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/\s+/g, " ")
    .trim();
}

function roundM2(m2: unknown): number {
  const n = toNumber(m2);
  return n == null ? 0 : Math.round(n);
}

/**
 * Stable cross-portal dedup key. Lowercased, diacritics-stripped:
 * `postal|street|houseNo|round(m2)|rooms[|floor]`.
 */
export function fingerprint(
  postal: unknown,
  street: unknown,
  houseNo: unknown,
  m2: unknown,
  rooms: unknown,
  floor?: unknown,
): string {
  const parts = [
    asciiFold(postal),
    asciiFold(street),
    asciiFold(houseNo),
    String(roundM2(m2)),
    asciiFold(rooms),
  ];
  if (floor !== undefined && floor !== null && asciiFold(floor) !== "") {
    parts.push(asciiFold(floor));
  }
  return parts.join("|");
}

/** Best-effort number coercion: accepts `"123 456,5 €"`, `"1.234,56"`, numbers. */
export function toNumber(v: unknown): number | null {
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

/** Treat a non-positive value as "no data" (e.g. Oikotie price-on-request = 0). */
function positiveOrNull(n: number | null): number | null {
  return n != null && n > 0 ? n : null;
}

/** No habitable Finnish property sells below this; lower values are auction
 *  starting-bid / "by offer" placeholders (notably from Etuovi's thin listpage). */
const MIN_REAL_PRICE_EUR = 1000;

/** A real asking price, or null when it's a sub-€1000 placeholder = price-on-request. */
function realPriceOrNull(n: number | null): number | null {
  return n != null && n >= MIN_REAL_PRICE_EUR ? n : null;
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

const PROPERTY_TYPE_MAP: Array<[RegExp, string]> = [
  [/omakotitalo|erillistalo|okt\b/, "omakotitalo"],
  [/paritalo/, "paritalo"],
  [/rivitalo/, "rivitalo"],
  [/(loma|mokki|mökki|vapaa-ajan)/, "mokki"],
  [/kerrostalo/, "kerrostalo"],
  [/maatila|tila\b/, "maatila"],
];

/** Etuovi listpage `propertySubtype` English enums → Finnish type tokens. */
const ENGLISH_TYPE_ENUMS: Record<string, string> = {
  detached_house: "omakotitalo",
  separate_house: "erillistalo",
  semi_detached_house: "paritalo",
  row_house: "rivitalo",
  cottage: "mokki",
  apartment_house: "kerrostalo",
};

export function normalizePropertyType(raw: unknown): string | null {
  const s = asciiFold(raw);
  if (s === "") return null;
  const englishKey = s.replace(/[\s-]+/g, "_");
  const mapped = ENGLISH_TYPE_ENUMS[englishKey];
  if (mapped) return mapped;
  for (const [re, val] of PROPERTY_TYPE_MAP) {
    if (re.test(s)) return val;
  }
  return s;
}

export function normalizeHoldingForm(raw: unknown): string | null {
  const s = asciiFold(raw);
  if (s === "") return null;
  if (/kiinteisto/.test(s)) return "kiinteisto";
  if (/(osake|asunto-osake|asunto osake|huoneisto)/.test(s)) return "asunto_osake";
  if (/maaraala/.test(s)) return "maaraala";
  if (/hallinnanjako/.test(s)) return "hallinnanjako";
  return s;
}

export function normalizeHeatingType(raw: unknown): string | null {
  let s = asciiFold(raw);
  if (s === "") return null;
  // "Ready-for" / "reserved" / removed heat is NOT an installed plant — strip the heat
  // token together with its qualifier so a mere conduit ("ilmavesilämpöpumppuvalmius")
  // or a decommissioned boiler ("öljylämmitys purettu") isn't recorded as the heating,
  // which would let a summer cabin read as year-round.
  s = s
    .replace(/[a-z]*lampopumppu(valmius|varaus)/g, " ")
    .replace(/(oljy|maalampo|kaukolampo|sahko|[a-z]*lampo)(valmius|varaus)/g, " ")
    .replace(/(oljy\w*|maalampo\w*|kaukolampo\w*|[a-z]*lampopumppu\w*)\s+\w*\s*(purett|poistett)\w*/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  if (s === "") return null;
  if (/kaukolampo/.test(s)) return "kaukolampo";
  if (/maalampo/.test(s)) return "maalampo";
  if (/oljy/.test(s)) return "oljy";
  if (/(ilmavesilampopumppu|ivlp|ilma-?vesi)/.test(s)) return "ivlp";
  if (/(ilmalampopumppu|ilma-?ilma)/.test(s)) return "sahko";
  // Explicit electric heating system wins over a supplemental fireplace/stove
  // ("Sähkölämmitys, Uuni- tai takkalämmitys" = electric primary + wood backup).
  if (/sahkolammit|sahkolampo/.test(s)) return "sahko";
  if (/puu|takka|halko|klapi/.test(s)) return "puu";
  if (/sahko/.test(s)) return "sahko";
  return s;
}

export function normalizeShore(raw: unknown): string | null {
  const s = asciiFold(raw);
  if (s === "") return null;
  if (/oma\s*ranta|omarant/.test(s)) return "oma_ranta";
  if (/rantaoikeus|ranta-oikeus|yhteisrant|yhteinen ranta/.test(s)) return "rantaoikeus";
  if (/ei\s*rantaa|ei_rantaa|ranta:?\s*ei/.test(s)) return "ei_rantaa";
  return null;
}

export function normalizePlotOwnership(raw: unknown): string | null {
  const s = asciiFold(raw);
  if (s === "") return null;
  if (/vuokra/.test(s)) return "vuokra";
  if (/oma/.test(s)) return "oma";
  return s;
}

export function normalizeEnergyClass(raw: unknown): string | null {
  const s = String(raw ?? "").trim().toUpperCase();
  const m = s.match(/\b([A-G])\b/);
  return m ? (m[1] ?? null) : null;
}

const RISK_TOKENS: Array<[RegExp, string]> = [
  [/valesokkeli|vale-?sokkeli/, "valesokkeli"],
  [/kaksoislaatta|kaksois-?laatta/, "kaksoislaatta"],
  [/kosteusvaurio|kosteus-?vaurio/, "kosteusvaurio"],
  [/homevaurio|home\b|homevaur/, "homevaurio"],
  [/salaoja|salaojat|salaojitus/, "salaoja"],
  [/asbesti/, "asbesti"],
  [/kreosootti/, "kreosootti"],
  [/putkiremontti|putki-?remontti|linjasaneeraus/, "putkiremontti"],
  [/mikrobivaurio|mikrobi/, "mikrobivaurio"],
  [/kattovuoto|vesivahinko|vesivaurio/, "vesivaurio"],
  [/oljysailio|oljysaili/, "oljysailio"],
];

/** Scan free text for known Finnish risk-structure tokens. Order-stable, deduped. */
export function extractRiskStructures(text: unknown): string[] {
  const s = asciiFold(text);
  if (s === "") return [];
  const found: string[] = [];
  for (const [re, token] of RISK_TOKENS) {
    if (re.test(s) && !found.includes(token)) {
      found.push(token);
    }
  }
  return found;
}

const HASH_FIELDS: Array<keyof NormalizedListing> = [
  "price_eur",
  "debt_free_price_eur",
  "debt_share_eur",
  "status",
  "living_area_m2",
  "room_count",
  "year_built",
  "property_type",
  "holding_form",
  "heating_type",
  "energy_class",
  "shore",
  "plot_ownership",
  "address",
  "municipality",
  "postal_code",
];

/**
 * FNV-1a 32-bit hex of the change-relevant normalized fields. Stable across
 * runs for identical input; used for cheap "did anything change" detection.
 */
export function contentHash(row: Partial<NormalizedListing>): string {
  const parts: string[] = [];
  for (const field of HASH_FIELDS) {
    const v = row[field];
    parts.push(`${field}=${Array.isArray(v) ? v.join(",") : v ?? ""}`);
  }
  const s = parts.join("|");
  let h = 0x811c9dc5;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return (h >>> 0).toString(16).padStart(8, "0");
}

function splitAddress(address: string | null): { street: string | null; houseNo: string | null } {
  if (!address) return { street: null, houseNo: null };
  const m = address.match(/^(.*?)(\d+[a-zA-Z]?)\s*$/);
  if (m) {
    return { street: (m[1] ?? "").trim() || null, houseNo: (m[2] ?? "").trim() || null };
  }
  return { street: address.trim() || null, houseNo: null };
}

/**
 * Build a full NormalizedListing from a partial object, defaulting every absent
 * field. Lets ingestion that runs OUTSIDE the Worker (on a residential IP, e.g.
 * Swedish Booli / Norwegian Finn, which the datacenter IP can't reach) POST
 * already-normalized rows to `/api/import-normalized` and have them upserted.
 */
export function coerceNormalized(raw: Record<string, unknown>): NormalizedListing {
  const s = (k: string): string | null => (typeof raw[k] === "string" && raw[k] !== "" ? (raw[k] as string) : null);
  const n = (k: string): number | null => (typeof raw[k] === "number" && Number.isFinite(raw[k]) ? (raw[k] as number) : null);
  return {
    portal: s("portal") ?? "unknown",
    portal_listing_id: s("portal_listing_id") ?? "",
    url: s("url") ?? "",
    country: normalizeCountry(s("country")) ?? "FI",
    property_type: s("property_type"),
    holding_form: s("holding_form"),
    kiinteistotunnus: s("kiinteistotunnus"),
    address: s("address"),
    municipality: s("municipality"),
    postal_code: s("postal_code"),
    district: s("district"),
    lat: n("lat"),
    lon: n("lon"),
    price_eur: n("price_eur"),
    debt_free_price_eur: n("debt_free_price_eur"),
    debt_share_eur: n("debt_share_eur"),
    price_per_m2: n("price_per_m2"),
    maintenance_charge_eur: n("maintenance_charge_eur"),
    financing_charge_eur: n("financing_charge_eur"),
    ground_rent_eur_yr: n("ground_rent_eur_yr"),
    living_area_m2: n("living_area_m2"),
    total_area_m2: n("total_area_m2"),
    plot_area_m2: n("plot_area_m2"),
    room_count: n("room_count"),
    room_layout: s("room_layout"),
    floors: n("floors"),
    year_built: n("year_built"),
    occupancy_year: n("occupancy_year"),
    roof_year: n("roof_year"),
    pipes_renovated_year: n("pipes_renovated_year"),
    water_body: s("water_body"),
    kiinteistovero_eur_yr: n("kiinteistovero_eur_yr"),
    electricity_eur_yr: n("electricity_eur_yr"),
    condition_class: s("condition_class"),
    inspection_status: s("inspection_status"),
    frame_material: s("frame_material"),
    facade_material: s("facade_material"),
    roof_material: s("roof_material"),
    energy_class: s("energy_class"),
    e_value: n("e_value"),
    risk_structures: Array.isArray(raw.risk_structures) ? (raw.risk_structures as string[]) : [],
    plot_ownership: s("plot_ownership"),
    lease_end_year: n("lease_end_year"),
    shore: s("shore"),
    shore_sauna: n("shore_sauna"),
    heating_type: s("heating_type"),
    heat_distribution: s("heat_distribution"),
    water_supply: s("water_supply"),
    sewer_system: s("sewer_system"),
    broadband: s("broadband"),
    sauna: s("sauna"),
    parking: s("parking"),
    road_access: s("road_access"),
    intended_use: s("intended_use"),
    zoning_status: s("zoning_status"),
    description: s("description"),
    status: s("status") ?? "active",
    raw_json: typeof raw.raw_json === "string" ? (raw.raw_json as string) : JSON.stringify(raw),
  };
}

export function fingerprintFor(row: NormalizedListing): string {
  const { street, houseNo } = splitAddress(row.address);
  // Country-prefix so two same-named addresses in different countries never
  // dedup together (matches migration 0007's 'FI|' backfill).
  const country = normalizeCountry(row.country) ?? "FI";
  return (
    `${country}|` +
    fingerprint(row.postal_code, street, houseNo, row.living_area_m2, row.room_count, row.floors)
  );
}

/**
 * Map an Oikotie `/api/cards` card object into a NormalizedListing. The card
 * schema drifts and fields may be missing — every access is defensive.
 */
/** Oikotie buildingData.buildingType integer code → Finnish type. Codes can drift. */
/**
 * Oikotie `buildingType` bitmask code → Finnish type. Codes verified live against
 * asunnot.oikotie.fi/api/cards (buildingType[]=N returns cards whose buildingData
 * carries the same N): 4=omakotitalo, 8=vapaa-ajan/mökki, 32=erillistalo,
 * 64=paritalo. Matches tui ingest.rs `building_type_codes`.
 */
function oikotieBuildingType(code: unknown): string | undefined {
  switch (code) {
    case 1:
      return "kerrostalo";
    case 2:
      return "rivitalo";
    case 4:
      return "omakotitalo";
    case 8:
      return "mökki";
    case 32:
      return "erillistalo";
    case 64:
      return "paritalo";
    case 256:
      return "luhtitalo";
    default:
      return undefined;
  }
}

export function normalizeOikotieCard(card: unknown): NormalizedListing {
  const c = (card ?? {}) as Record<string, unknown>;
  const id = firstString(c["id"], c["cardId"], get(c, "card.id")) ?? "";
  const url =
    firstString(c["url"], get(c, "links.self")) ??
    (id ? `https://asunnot.oikotie.fi/myytavat-asunnot/${id}` : "https://asunnot.oikotie.fi/");

  const description = firstString(c["description"], c["shortDescription"], c["text"]) ?? "";
  const buildingTypeName =
    oikotieBuildingType(get(c, "buildingData.buildingType")) ??
    firstString(c["buildingType"], get(c, "buildingData.type"));
  const address = firstString(
    c["address"],
    get(c, "buildingData.address"),
    get(c, "location.address"),
    c["streetAddress"],
  );

  const priceText = firstString(c["price"], get(c, "data.price"));
  const visualType = firstString(c["visualType"], c["cardType"]);
  const status = mapOikotieStatus(c["status"], visualType);

  const row: NormalizedListing = {
    portal: "oikotie",
    portal_listing_id: id,
    url,
    country: normalizeCountry(oikotieCountry(card)) ?? "FI",
    property_type: normalizePropertyType(buildingTypeName ?? description),
    holding_form: normalizeHoldingForm(
      firstString(c["holdingType"], c["ownershipType"], get(c, "data.holdingType")),
    ),
    kiinteistotunnus: firstString(c["propertyIdentifier"], c["kiinteistotunnus"]),
    address,
    municipality: firstString(
      get(c, "buildingData.city"),
      c["city"],
      get(c, "location.city"),
      c["municipality"],
      get(c, "location.municipality"),
    ),
    postal_code: firstString(c["postalCode"], get(c, "location.postalCode"), c["zipCode"]),
    district: firstString(get(c, "buildingData.district"), c["district"], get(c, "location.district")),
    lat: toNumber(firstString(get(c, "coordinates.latitude"), get(c, "location.lat"), c["latitude"])),
    lon: toNumber(firstString(get(c, "coordinates.longitude"), get(c, "location.lng"), c["longitude"])),
    price_eur: realPriceOrNull(toInt(priceText)),
    debt_free_price_eur: toInt(firstString(c["debtFreePrice"], get(c, "data.debtFreePrice"))),
    debt_share_eur: toInt(firstString(c["debtShare"], get(c, "data.debt"))),
    price_per_m2: positiveOrNull(toNumber(firstString(c["pricePerSquare"], c["pricePerM2"]))),
    maintenance_charge_eur: toInt(c["maintenanceCharge"]),
    financing_charge_eur: toInt(c["financingCharge"]),
    ground_rent_eur_yr: toInt(c["groundRent"]),
    living_area_m2: toNumber(firstString(c["size"], c["area"], get(c, "data.area"))),
    total_area_m2: toNumber(c["totalArea"]),
    plot_area_m2: toNumber(firstString(c["sizeLot"], c["plotArea"], c["lotArea"])),
    room_count: toNumber(firstString(c["rooms"], c["roomCount"])),
    room_layout: firstString(c["roomConfiguration"], c["roomLayout"]),
    floors: toNumber(firstString(get(c, "buildingData.floorCount"), c["floor"])),
    year_built: toInt(firstString(get(c, "buildingData.year"), c["buildYear"], c["yearOfBuilding"], get(c, "data.buildYear"))),
    occupancy_year: toInt(c["occupancyYear"]),
    roof_year: null,
    pipes_renovated_year: null,
    water_body: null,
    kiinteistovero_eur_yr: null,
    electricity_eur_yr: null,
    condition_class: firstString(c["condition"], c["conditionClass"]),
    inspection_status: firstString(c["inspectionStatus"]),
    frame_material: firstString(c["frameMaterial"]),
    facade_material: firstString(c["facadeMaterial"]),
    roof_material: firstString(c["roofMaterial"]),
    energy_class: normalizeEnergyClass(firstString(c["energyClass"], get(c, "data.energyClass"))),
    e_value: toNumber(c["eValue"]),
    risk_structures: extractRiskStructures(description),
    plot_ownership: normalizePlotOwnership(firstString(c["lotOwnership"], c["plotOwnership"])),
    lease_end_year: toInt(c["leaseEndYear"]),
    shore: normalizeShore(firstString(c["shore"], c["beach"], c["waterfront"])),
    shore_sauna: boolToInt(c["shoreSauna"]),
    heating_type: normalizeHeatingType(firstString(c["heating"], get(c, "data.heating"))),
    heat_distribution: firstString(c["heatDistribution"]),
    water_supply: firstString(c["waterSupply"]),
    sewer_system: firstString(c["sewer"], c["sewerSystem"]),
    broadband: firstString(c["broadband"]),
    sauna: firstString(c["sauna"]),
    parking: firstString(c["parking"]),
    road_access: firstString(c["roadAccess"]),
    intended_use: firstString(c["intendedUse"]),
    zoning_status: firstString(c["zoning"], c["zoningStatus"]),
    description: description || null,
    status,
    raw_json: safeStringify(card),
  };
  applyOikotieDetail(row, c);
  return row;
}

/** Normalize a kuntoluokka label to a canonical class (Kunto: Hyvä → "hyvä"). */
export function normalizeConditionClass(raw: unknown): string | null {
  const s = asciiFold(raw);
  if (s === "") return null;
  if (/erinomai/.test(s)) return "erinomainen";
  if (/hyva/.test(s)) return "hyvä";
  if (/tyydyttav/.test(s)) return "tyydyttävä";
  if (/valttav/.test(s)) return "välttävä";
  if (/huono/.test(s)) return "huono";
  if (/uudis/.test(s)) return "uudiskohde";
  return null;
}

/** Normalize the Oikotie water-body label ("Rannan (vesistön) tyyppi"). */
export function normalizeWaterBody(raw: unknown): string | null {
  const s = asciiFold(raw);
  if (s === "") return null;
  if (/jarvi/.test(s)) return "jarvi";
  if (/joki/.test(s)) return "joki";
  if (/meri/.test(s)) return "meri";
  if (/lampi/.test(s)) return "lampi";
  return null;
}

/** Euro amount from an Oikotie figure like "317,01 € / v" or "1 234 € / v".
 *  Uses toNumber so Finnish dot-thousands/comma-decimals ("1.234,56") parse right. */
function euroAmount(raw: unknown): number | null {
  if (typeof raw !== "string") return null;
  const n = toNumber(raw);
  return n == null ? null : Math.round(n);
}

/**
 * A "X € / kk" or "X € / v" euro figure normalized to euros per YEAR. Requires an
 * explicit euro token and rejects energy units — the source field
 * "Keskimääräinen sähkönkulutus" is sometimes kWh, which must NOT be read as euros.
 */
function euroPerYear(raw: unknown): number | null {
  if (typeof raw !== "string" || !/€|eur/i.test(raw) || /kwh|kw·h/i.test(raw)) return null;
  const eur = euroAmount(raw);
  if (eur === null || eur <= 0) return null;
  return /\/\s*kk|kuukau/i.test(raw) ? eur * 12 : eur;
}

/** First 4-digit year in a string (e.g. a renovation note "Kattoremontti 2023"). */
function firstYear(raw: unknown): number | null {
  const m = typeof raw === "string" ? raw.match(/\b(19|20)\d{2}\b/) : null;
  return m ? Number(m[0]) : null;
}

/**
 * Year of an actual roof RENEWAL. A bare year (the "Kattoremontti" row label
 * already implies renovation) or an explicit re-roofing counts; painting alone
 * ("katon maalaus 2020") is upkeep, not a renewal, so it must NOT reset the
 * roof's age clock and mask a roof that is in fact decades old.
 */
function roofRenovationYear(raw: unknown): number | null {
  if (typeof raw !== "string") return null;
  const year = firstYear(raw);
  if (year === null) return null;
  const s = raw.toLowerCase();
  const paintedOnly = /maala/.test(s) && !/(uusi|vaihd|asenne|peruskorj|remont|pinnoit|huovan|katteen|kate)/.test(s);
  return paintedOnly ? null : year;
}

/**
 * Latest year of an actual plumbing/sewer RENOVATION in the renovations text.
 * Only a segment that has a plumbing keyword AND a renovation verb (and is not a
 * build-year statement) qualifies, and the year nearest the plumbing keyword is
 * taken — so "Rakennusvuosi 2016 … lvi" or "remontoitu 2021 … putket uusittu 2005"
 * don't leak the build/unrelated year into the risk model.
 */
function pipeRenovationYear(raw: unknown): number | null {
  if (typeof raw !== "string") return null;
  // `putk[ie]` catches the plural nominative "putket" (the most common way pipes are
  // named), not just "putki".
  const PIPE = /(viemär|putk[ie]|jätevesi|käyttövesi|vesijohto|lvi[- ]?(saneer|remont|uusi))/iu;
  // Suffixed "uusi-" forms (uusittu / uusiminen / uusinta / uusiksi) — deliberately
  // NOT the bare adjective "uusi" (which would read "uusi vesijohtoverkosto" = a NEW
  // network nearby as a renewal) nor the infinitive "uusia" (a future need). `remont`
  // also catches the noun "putkiremontti".
  const RENO = /(uusi(tt|ta|min|mis|nt|ks)|remont|saneerat|uudistet|asennett|peruskorjat)/iu;
  const BUILD = /(rakennusvuosi|rakennettu|valmistunut)/iu;
  // A renovation that is NEEDED / planned / still original is not one that was DONE —
  // don't read "putket uusittava", "alkuperäiset putket" or "putkiremontti edessä"
  // as a completed renewal year.
  const NEED = /(uusittav|tarpeess|tulee uusi|on uusittav|suunnitteil|edess|alkuperäi)/iu;
  let best: number | null = null;
  for (const seg of raw.split(/[.,;\n]/)) {
    if (BUILD.test(seg) || NEED.test(seg) || !RENO.test(seg)) continue;
    const pipe = seg.search(PIPE);
    if (pipe < 0) continue;
    let nearest: number | null = null;
    let nearestDist = Infinity;
    for (const m of seg.matchAll(/\b(19|20)\d{2}\b/g)) {
      const dist = Math.abs((m.index ?? 0) - pipe);
      if (dist < nearestDist) {
        nearestDist = dist;
        nearest = Number(m[0]);
      }
    }
    if (nearest !== null && (best === null || nearest > best)) best = nearest;
  }
  return best;
}

/**
 * Fold the per-listing detail-page info-table (`c.details`) and full description
 * (`c.fullDescription`) into the row — these are richer and more reliable than the
 * search-card fields, so detail values win when present.
 */
function applyOikotieDetail(row: NormalizedListing, c: Record<string, unknown>): void {
  const det = (c["details"] ?? null) as Record<string, unknown> | null;
  const full = firstString(c["fullDescription"]);
  if (full && full.length > (row.description?.length ?? 0)) row.description = full;
  // Facts that live in the prose (not the info-table): extract them even when the
  // detail page carries no structured table at all. A structured value, when
  // present, overrides the prose-derived one below.
  const prosePipeYear = pipeRenovationYear(full);
  if (prosePipeYear !== null) row.pipes_renovated_year = prosePipeYear;
  // Only the explicit "oma ranta" literal asserts owned shore; a rantasauna can sit
  // on a shared shore, so it must not coerce shore ownership.
  if (/\boma ranta\b/i.test(full ?? "") && !row.shore) row.shore = "oma_ranta";
  if (!det) return;
  const dv = (label: string) => firstString(det[label]);
  const set = <K extends keyof NormalizedListing>(k: K, v: NormalizedListing[K] | null) => {
    if (v !== null && v !== undefined && v !== "") row[k] = v as NormalizedListing[K];
  };
  set("condition_class", normalizeConditionClass(dv("Kunto")));
  set("shore", normalizeShore(dv("Rannan omistus")));
  set("water_body", normalizeWaterBody(dv("Rannan (vesistön) tyyppi")));
  set("heating_type", normalizeHeatingType([dv("Lisätietoja lämmityksestä"), dv("Lämmitys")].filter(Boolean).join(" ")));
  set("plot_ownership", normalizePlotOwnership(dv("Tontin omistus")));
  set("energy_class", normalizeEnergyClass(dv("Energialuokka")));
  set("frame_material", dv("Rakennusmateriaali"));
  set("roof_material", dv("Kattomateriaali"));
  // "Kunnallistekniikka" is combined municipal infra ("Vesi, Sähkö, Viemäri"): split it.
  const muni = dv("Kunnallistekniikka");
  if (muni) {
    if (/vesi|vesijohto/i.test(muni)) set("water_supply", "kunnallinen");
    if (/viemär/i.test(muni)) set("sewer_system", "kunnallinen viemäri");
  }
  set("year_built", toInt(dv("Rakennusvuosi")) ?? row.year_built);
  set("kiinteistovero_eur_yr", euroAmount(dv("Kiinteistövero")));
  set("electricity_eur_yr", euroPerYear(dv("Keskimääräinen sähkönkulutus")));
  set("roof_year", roofRenovationYear(dv("Kattoremontti")));
  // The structured renovations table, when it carries a pipe-renewal year, wins
  // over the prose-derived one set above.
  set("pipes_renovated_year", pipeRenovationYear(dv("Tehdyt remontit")));
}

/** The Nordic markets kontu covers. */
export const SUPPORTED_COUNTRIES = new Set(["FI", "SE", "NO", "DK", "IS"]);

/** Map a free-text country name or ISO code to a supported ISO code, or null. */
export function normalizeCountry(country: string | null): string | null {
  if (country == null || country.trim() === "") return null;
  const c = country.trim().toLowerCase();
  if (/^(fi|suomi|finland|finnland)$/.test(c)) return "FI";
  if (/^(se|ruotsi|sweden|sverige)$/.test(c)) return "SE";
  if (/^(no|norja|norway|norge)$/.test(c)) return "NO";
  if (/^(dk|tanska|denmark|danmark)$/.test(c)) return "DK";
  if (/^(is|islanti|iceland|ísland|island)$/.test(c)) return "IS";
  return null;
}

/** Country/region names that leak in as a "municipality" for listings abroad. */
const FOREIGN_MUNICIPALITIES = new Set([
  "viro", "eesti", "estonia", "espanja", "spain", "thaimaa", "thailand",
  "ranska", "france", "philippines", "filippiinit", "portugali", "portugal",
  "kreikka", "greece", "italia", "italy", "turkki", "turkey", "bulgaria",
  "unkari", "hungary", "kypros", "cyprus",
]);

/** Oikotie `buildingData.country` (absent for Etuovi). */
export function oikotieCountry(card: unknown): string | null {
  const c = (card ?? {}) as Record<string, unknown>;
  return firstString(get(c, "buildingData.country"), c["country"]);
}

/**
 * True when a listing is outside the supported Nordic markets — e.g. a Finnish
 * portal card located in Spain or Estonia. A listing that resolves to FI/SE/NO/
 * DK/IS is kept; only genuinely-foreign stock is dropped.
 */
export function isForeignListing(municipality: string | null, country: string | null): boolean {
  const iso = normalizeCountry(country);
  if (iso != null) return !SUPPORTED_COUNTRIES.has(iso);
  // No usable country: a named-but-unrecognised one is abroad; otherwise fall
  // back to the municipality (Finnish portals leak a country name as the city).
  if (country != null && country.trim() !== "") return true;
  return FOREIGN_MUNICIPALITIES.has(asciiFold(municipality));
}

/** Cover-image URL(s) for an Oikotie card (already absolute https CDN links). */
export function oikotiePhotoUrls(card: unknown): string[] {
  const c = (card ?? {}) as Record<string, unknown>;
  const u = firstString(
    get(c, "images.wide"),
    get(c, "imageUrl.wide"),
    get(c, "images.url"),
    c["image"],
  );
  return u && /^https?:\/\//i.test(u) ? [u] : [];
}

/**
 * Cover-image URL for an Etuovi announcement. `mainImageUri` is protocol-relative
 * with a `{imageParameters}` size placeholder; resolve both. Empty when hidden.
 */
export function etuoviPhotoUrls(announcement: unknown): string[] {
  const a = (announcement ?? {}) as Record<string, unknown>;
  if (a["mainImageHidden"] === true) return [];
  const raw = firstString(a["mainImageUri"], a["imageUri"], a["coverImageUri"]);
  if (!raw) return [];
  const withScheme = raw.startsWith("//") ? `https:${raw}` : raw;
  const resolved = withScheme.replace("{imageParameters}", "1600x1066");
  return /^https?:\/\//i.test(resolved) ? [resolved] : [];
}

/** Parse the leading integer of an Etuovi `roomCount` (e.g. "3 huonetta" → 3). */
function leadingInt(v: unknown): number | null {
  if (v == null) return null;
  if (typeof v === "number") return Number.isFinite(v) ? Math.trunc(v) : null;
  const m = String(v).match(/-?\d+/);
  return m ? Number(m[0]) : null;
}

/**
 * Last whitespace-separated token of an Etuovi `addressLine2` (the municipality),
 * treating known multi-word kunta suffixes ("X kunta", "Koski Tl") as one name.
 */
function lastToken(v: unknown): string | null {
  const s = typeof v === "string" ? v.trim() : "";
  if (s === "") return null;
  const parts = s.split(/\s+/);
  const last = parts[parts.length - 1];
  if (last === undefined) return null;
  if (parts.length >= 2) {
    const lastLower = last.toLowerCase();
    if (lastLower === "kunta" || lastLower === "tl") {
      return `${parts[parts.length - 2]} ${last}`;
    }
  }
  return last;
}

/** `addressLine2` minus its last token (the district), or null if nothing remains. */
function withoutLastToken(v: unknown): string | null {
  const s = typeof v === "string" ? v.trim() : "";
  if (s === "") return null;
  const parts = s.split(/\s+/);
  if (parts.length <= 1) return null;
  return parts.slice(0, -1).join(" ") || null;
}

/**
 * Map a live Etuovi `listpage` announcement into a NormalizedListing. Field
 * mapping verified against the real listpage shape; every access is defensive
 * and the function never throws on missing/garbage input.
 */
export function normalizeEtuoviAnnouncement(announcement: unknown): NormalizedListing {
  const a = (announcement ?? {}) as Record<string, unknown>;
  const friendlyId = firstString(a["friendlyId"]);
  const id = friendlyId ?? firstString(a["id"], a["announcementId"]) ?? "";
  const url = friendlyId
    ? `https://www.etuovi.com/kohde/${friendlyId}`
    : (firstString(a["url"], a["link"]) ?? "https://www.etuovi.com/");

  const description = firstString(a["searchListItemText"], a["description"]) ?? "";
  const addressLine2 = a["addressLine2"];

  const row: NormalizedListing = {
    portal: "etuovi",
    portal_listing_id: id,
    url,
    country: "FI",
    property_type: normalizePropertyType(firstString(a["propertySubtype"]) ?? description),
    holding_form: normalizeHoldingForm(
      firstString(a["holdingType"], a["ownershipType"], get(a, "property.holdingType")),
    ),
    kiinteistotunnus: firstString(a["propertyIdentifier"], a["kiinteistotunnus"]),
    address: firstString(a["addressLine1"]),
    municipality: lastToken(addressLine2),
    postal_code: firstString(a["postalCode"], get(a, "address.postalCode")),
    district: withoutLastToken(addressLine2),
    lat: toNumber(firstString(a["latitude"], get(a, "coordinates.latitude"))),
    lon: toNumber(firstString(a["longitude"], get(a, "coordinates.longitude"))),
    price_eur: realPriceOrNull(toInt(firstString(a["searchPrice"], a["price"], a["sellingPrice"]))),
    debt_free_price_eur: toInt(firstString(a["debtFreePrice"], a["unencumberedSalesPrice"])),
    debt_share_eur: toInt(firstString(a["debtShare"], a["shareOfLiabilities"])),
    price_per_m2: positiveOrNull(toNumber(firstString(a["pricePerSquareMeter"], a["pricePerM2"]))),
    maintenance_charge_eur: toInt(firstString(a["maintenanceCharge"], a["careCharge"])),
    financing_charge_eur: toInt(a["financingCharge"]),
    ground_rent_eur_yr: toInt(a["groundRent"]),
    living_area_m2: toNumber(firstString(a["area"], a["livingArea"])),
    total_area_m2: toNumber(firstString(a["totalArea"], a["overallArea"])),
    plot_area_m2: toNumber(firstString(a["lotArea"], a["plotArea"])),
    room_count: leadingInt(a["roomCount"]),
    room_layout: firstString(a["roomStructure"], a["roomLayout"]),
    floors: toNumber(firstString(a["residentialFloorCount"], a["floor"], a["numberOfFloors"])),
    year_built: toInt(firstString(a["constructionFinishedYear"], a["constructionYear"], a["yearBuilt"])),
    occupancy_year: toInt(a["occupancyYear"]),
    roof_year: null,
    pipes_renovated_year: null,
    water_body: null,
    kiinteistovero_eur_yr: null,
    electricity_eur_yr: null,
    condition_class: firstString(a["condition"], a["conditionClassType"]),
    inspection_status: firstString(a["inspectionStatus"]),
    frame_material: firstString(a["frameMaterial"]),
    facade_material: firstString(a["facadeMaterial"]),
    roof_material: firstString(a["roofMaterial"], a["roofType"]),
    energy_class: normalizeEnergyClass(firstString(a["energyClass"], get(a, "property.energyClass"))),
    e_value: toNumber(a["eValue"]),
    risk_structures: extractRiskStructures(description),
    plot_ownership: normalizePlotOwnership(firstString(a["lotHolding"], a["plotOwnership"], a["lotOwnershipType"])),
    lease_end_year: toInt(a["leaseEndYear"]),
    shore: normalizeShore(firstString(a["shore"], a["beachType"], a["waterfront"])),
    shore_sauna: boolToInt(a["beachSauna"] ?? a["shoreSauna"]),
    heating_type: normalizeHeatingType(firstString(a["heating"], a["heatingType"], get(a, "property.heating"))),
    heat_distribution: firstString(a["heatDistribution"]),
    water_supply: firstString(a["waterSupply"], a["water"]),
    sewer_system: firstString(a["sewer"], a["sewerSystem"]),
    broadband: firstString(a["broadband"], a["dataConnection"]),
    sauna: firstString(a["sauna"]),
    parking: firstString(a["parking"], a["parkingSpace"]),
    road_access: firstString(a["roadAccess"], a["access"]),
    intended_use: firstString(a["intendedUse"], a["usage"]),
    zoning_status: firstString(a["zoning"], a["planningSituation"]),
    description: description || null,
    status: mapEtuoviStatus(a["status"] ?? a["announcementState"]),
    raw_json: safeStringify(announcement),
  };
  return row;
}

function boolToInt(v: unknown): number | null {
  if (v == null) return null;
  if (typeof v === "boolean") return v ? 1 : 0;
  const s = asciiFold(v);
  if (s === "") return null;
  if (/^(true|kylla|yes|1|on)$/.test(s)) return 1;
  if (/^(false|ei|no|0|off)$/.test(s)) return 0;
  return null;
}

function mapOikotieStatus(status: unknown, visualType: unknown): string {
  const s = asciiFold(status);
  if (/sold|myyty/.test(s)) return "sold";
  if (/reserved|varattu/.test(s)) return "reserved";
  if (/withdrawn|poistettu/.test(s)) return "withdrawn";
  if (asciiFold(visualType) === "sold") return "sold";
  return "active";
}

function mapEtuoviStatus(status: unknown): string {
  const s = asciiFold(status);
  if (/sold|myyty/.test(s)) return "sold";
  if (/reserved|varattu/.test(s)) return "reserved";
  if (/(withdrawn|removed|poistettu|expired)/.test(s)) return "withdrawn";
  return "active";
}

function safeStringify(v: unknown): string {
  try {
    return JSON.stringify(v ?? {});
  } catch {
    return "{}";
  }
}
