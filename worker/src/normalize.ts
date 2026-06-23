/**
 * Pure normalization layer. No network, no D1 — every function here must be
 * deterministic and total (never throw on missing/garbage source data) so it can
 * be exhaustively unit-tested and reused from the crawler and from tests alike.
 */

export interface NormalizedListing {
  portal: string;
  portal_listing_id: string;
  url: string;

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

export function normalizePropertyType(raw: unknown): string | null {
  const s = asciiFold(raw);
  if (s === "") return null;
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
  const s = asciiFold(raw);
  if (s === "") return null;
  if (/kaukolampo/.test(s)) return "kaukolampo";
  if (/maalampo/.test(s)) return "maalampo";
  if (/oljy/.test(s)) return "oljy";
  if (/(ilmavesilampopumppu|ivlp)/.test(s)) return "ilmalampopumppu";
  if (/(ilmalampopumppu|ilma-vesi)/.test(s)) return "ilmalampopumppu";
  if (/puu|takka|halko|klapi/.test(s)) return "puu";
  if (/sahko/.test(s)) return "sahko";
  return s;
}

export function normalizeShore(raw: unknown): string | null {
  const s = asciiFold(raw);
  if (s === "") return null;
  if (/oma\s*ranta|omarant/.test(s)) return "oma_ranta";
  if (/rantaoikeus|ranta-oikeus/.test(s)) return "rantaoikeus";
  if (/ei\s*rantaa|ei_rantaa|ranta:?\s*ei/.test(s)) return "ei_rantaa";
  return s;
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

export function fingerprintFor(row: NormalizedListing): string {
  const { street, houseNo } = splitAddress(row.address);
  return fingerprint(
    row.postal_code,
    street,
    houseNo,
    row.living_area_m2,
    row.room_count,
    row.floors,
  );
}

/**
 * Map an Oikotie `/api/cards` card object into a NormalizedListing. The card
 * schema drifts and fields may be missing — every access is defensive.
 */
export function normalizeOikotieCard(card: unknown): NormalizedListing {
  const c = (card ?? {}) as Record<string, unknown>;
  const id = firstString(c["id"], c["cardId"], get(c, "card.id")) ?? "";
  const url =
    firstString(c["url"], get(c, "links.self")) ??
    (id ? `https://asunnot.oikotie.fi/myytavat-asunnot/${id}` : "https://asunnot.oikotie.fi/");

  const description = firstString(c["description"], c["shortDescription"], c["text"]) ?? "";
  const rawType = firstString(c["buildingType"], get(c, "buildingData.type"), c["roomConfiguration"]);
  const address = firstString(c["address"], get(c, "location.address"), c["streetAddress"]);

  const priceText = firstString(c["price"], get(c, "data.price"));
  const visualType = firstString(c["visualType"], c["cardType"]);
  const status = mapOikotieStatus(c["status"], visualType);

  const row: NormalizedListing = {
    portal: "oikotie",
    portal_listing_id: id,
    url,
    property_type: normalizePropertyType(rawType ?? description),
    holding_form: normalizeHoldingForm(
      firstString(c["holdingType"], c["ownershipType"], get(c, "data.holdingType")),
    ),
    kiinteistotunnus: firstString(c["propertyIdentifier"], c["kiinteistotunnus"]),
    address,
    municipality: firstString(
      c["city"],
      get(c, "location.city"),
      c["municipality"],
      get(c, "location.municipality"),
    ),
    postal_code: firstString(c["postalCode"], get(c, "location.postalCode"), c["zipCode"]),
    district: firstString(c["district"], get(c, "location.district")),
    lat: toNumber(firstString(get(c, "coordinates.latitude"), get(c, "location.lat"), c["latitude"])),
    lon: toNumber(firstString(get(c, "coordinates.longitude"), get(c, "location.lng"), c["longitude"])),
    price_eur: toInt(priceText),
    debt_free_price_eur: toInt(firstString(c["debtFreePrice"], get(c, "data.debtFreePrice"))),
    debt_share_eur: toInt(firstString(c["debtShare"], get(c, "data.debt"))),
    price_per_m2: toNumber(firstString(c["pricePerSquare"], c["pricePerM2"])),
    maintenance_charge_eur: toInt(c["maintenanceCharge"]),
    financing_charge_eur: toInt(c["financingCharge"]),
    ground_rent_eur_yr: toInt(c["groundRent"]),
    living_area_m2: toNumber(firstString(c["size"], c["area"], get(c, "data.area"))),
    total_area_m2: toNumber(c["totalArea"]),
    plot_area_m2: toNumber(firstString(c["plotArea"], c["lotArea"])),
    room_count: toNumber(firstString(c["rooms"], c["roomCount"])),
    room_layout: firstString(c["roomConfiguration"], c["roomLayout"]),
    floors: toNumber(c["floor"]),
    year_built: toInt(firstString(c["buildYear"], c["yearOfBuilding"], get(c, "data.buildYear"))),
    occupancy_year: toInt(c["occupancyYear"]),
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
    status,
    raw_json: safeStringify(card),
  };
  return row;
}

/**
 * Map an Etuovi announcement object into a NormalizedListing. Etuovi nests most
 * useful fields; this reads several candidate paths defensively.
 */
export function normalizeEtuoviAnnouncement(announcement: unknown): NormalizedListing {
  const a = (announcement ?? {}) as Record<string, unknown>;
  const id =
    firstString(a["friendlyId"], a["id"], a["announcementId"], get(a, "searchListItem.friendlyId")) ?? "";
  const url =
    firstString(a["url"], a["link"]) ??
    (id ? `https://www.etuovi.com/kohde/${id}` : "https://www.etuovi.com/");

  const description =
    firstString(a["description"], a["searchListItemText"], get(a, "property.description")) ?? "";
  const address = firstString(
    a["address"],
    get(a, "addressLine"),
    get(a, "property.address.streetAddress"),
    get(a, "searchListItem.address"),
  );

  const row: NormalizedListing = {
    portal: "etuovi",
    portal_listing_id: id,
    url,
    property_type: normalizePropertyType(
      firstString(a["propertyType"], a["realtyType"], get(a, "property.realtyType")) ?? description,
    ),
    holding_form: normalizeHoldingForm(
      firstString(a["holdingType"], a["ownershipType"], get(a, "property.holdingType")),
    ),
    kiinteistotunnus: firstString(a["propertyIdentifier"], a["kiinteistotunnus"]),
    address,
    municipality: firstString(
      a["city"],
      get(a, "property.address.postOffice"),
      get(a, "address.city"),
      a["municipality"],
    ),
    postal_code: firstString(
      a["postalCode"],
      get(a, "property.address.postalCode"),
      get(a, "address.postalCode"),
    ),
    district: firstString(a["district"], get(a, "property.address.district")),
    lat: toNumber(firstString(get(a, "coordinates.latitude"), get(a, "location.latitude"), a["latitude"])),
    lon: toNumber(firstString(get(a, "coordinates.longitude"), get(a, "location.longitude"), a["longitude"])),
    price_eur: toInt(firstString(a["price"], a["sellingPrice"], get(a, "property.price"))),
    debt_free_price_eur: toInt(firstString(a["debtFreePrice"], a["unencumberedSalesPrice"])),
    debt_share_eur: toInt(firstString(a["debtShare"], a["shareOfLiabilities"])),
    price_per_m2: toNumber(firstString(a["pricePerSquareMeter"], a["pricePerM2"])),
    maintenance_charge_eur: toInt(firstString(a["maintenanceCharge"], a["careCharge"])),
    financing_charge_eur: toInt(a["financingCharge"]),
    ground_rent_eur_yr: toInt(a["groundRent"]),
    living_area_m2: toNumber(firstString(a["area"], a["livingArea"], get(a, "property.area"))),
    total_area_m2: toNumber(firstString(a["totalArea"], a["overallArea"])),
    plot_area_m2: toNumber(firstString(a["lotArea"], a["plotArea"], get(a, "property.lotArea"))),
    room_count: toNumber(firstString(a["roomCount"], a["numberOfRooms"])),
    room_layout: firstString(a["roomLayout"], a["roomConfiguration"]),
    floors: toNumber(firstString(a["floor"], a["numberOfFloors"])),
    year_built: toInt(firstString(a["constructionYear"], a["yearBuilt"], get(a, "property.constructionYear"))),
    occupancy_year: toInt(a["occupancyYear"]),
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
