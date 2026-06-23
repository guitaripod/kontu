/**
 * Typed D1 access layer. Every query is parameterized and index-aware (D1 bills
 * rows-read). No HTTP, no portal logic here — just persistence primitives the
 * crawler, enrichment and API compose.
 */
import type { NormalizedListing } from "./normalize";
import { contentHash, fingerprintFor } from "./normalize";

export interface ListingRow {
  id: number;
  property_id: number | null;
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
  risk_structures: string | null;
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
  content_hash: string | null;
  first_seen: number;
  last_seen: number;
}

export type CostDefaults = Record<string, number>;

export interface CrawlStateRow {
  source: string;
  next_page: number;
  total_pages: number | null;
  cursor: string | null;
  status: string;
  last_tick: number | null;
  last_error: string | null;
  updated_at: number;
}

export interface SavedSearchRow {
  id: number;
  name: string;
  params_json: string;
  is_exact: number;
  created_at: number;
  last_run: number | null;
}

const now = (): number => Math.floor(Date.now() / 1000);

/** cost_defaults → flat `{ key: number }` (loan_term_years stays integer). */
export async function getCostDefaults(db: D1Database): Promise<CostDefaults> {
  const { results } = await db
    .prepare("SELECT key, num_value FROM cost_defaults")
    .all<{ key: string; num_value: number | null }>();
  const out: CostDefaults = {};
  for (const r of results) {
    if (r.num_value != null) out[r.key] = r.num_value;
  }
  return out;
}

export async function getSourceConfig(db: D1Database, source: string): Promise<Record<string, string>> {
  const { results } = await db
    .prepare("SELECT key, value FROM source_config WHERE source = ?")
    .bind(source)
    .all<{ key: string; value: string }>();
  const out: Record<string, string> = {};
  for (const r of results) out[r.key] = r.value;
  return out;
}

export async function setSourceConfig(
  db: D1Database,
  source: string,
  key: string,
  value: string,
): Promise<void> {
  await db
    .prepare(
      "INSERT INTO source_config (source, key, value, updated_at) VALUES (?, ?, ?, ?) " +
        "ON CONFLICT(source, key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
    )
    .bind(source, key, value, now())
    .run();
}

export interface ListingsFilter {
  municipality?: string;
  property_type?: string;
  holding_form?: string;
  price_min?: number;
  price_max?: number;
  m2_min?: number;
  m2_max?: number;
  rooms_min?: number;
  year_min?: number;
  shore?: string;
  heating_type?: string;
  energy_class_max?: string;
  plot_ownership?: string;
  max_days_on_market?: number;
  exclude?: string[];
  price_dropped?: boolean;
  text?: string;
  sort?: string;
  order?: "asc" | "desc";
  limit?: number;
  offset?: number;
}

const SORTABLE: Record<string, string> = {
  price: "l.price_eur",
  ppm2: "COALESCE(l.price_per_m2, CASE WHEN l.living_area_m2 > 0 THEN CAST(l.price_eur AS REAL)/l.living_area_m2 END)",
  size: "l.living_area_m2",
  year: "l.year_built",
  dom: "(-l.first_seen)",
  risk: "json_array_length(COALESCE(l.risk_structures,'[]'))",
  score: "sc.score",
};

const ENERGY_RANK: Record<string, number> = { A: 1, B: 2, C: 3, D: 4, E: 5, F: 6, G: 7 };

interface BuiltQuery {
  where: string;
  binds: unknown[];
}

/** Index-aware WHERE builder for the listings query. Exported for unit testing. */
export function buildListingsWhere(f: ListingsFilter): BuiltQuery {
  const clauses: string[] = [];
  const binds: unknown[] = [];

  if (f.municipality) {
    clauses.push("l.municipality = ? COLLATE NOCASE");
    binds.push(f.municipality);
  }
  if (f.property_type) {
    clauses.push("l.property_type = ?");
    binds.push(f.property_type);
  }
  if (f.holding_form) {
    clauses.push("l.holding_form = ?");
    binds.push(f.holding_form);
  }
  if (f.price_min != null) {
    clauses.push("l.price_eur >= ?");
    binds.push(f.price_min);
  }
  if (f.price_max != null) {
    clauses.push("l.price_eur <= ?");
    binds.push(f.price_max);
  }
  if (f.m2_min != null) {
    clauses.push("l.living_area_m2 >= ?");
    binds.push(f.m2_min);
  }
  if (f.m2_max != null) {
    clauses.push("l.living_area_m2 <= ?");
    binds.push(f.m2_max);
  }
  if (f.rooms_min != null) {
    clauses.push("l.room_count >= ?");
    binds.push(f.rooms_min);
  }
  if (f.year_min != null) {
    clauses.push("l.year_built >= ?");
    binds.push(f.year_min);
  }
  if (f.shore) {
    clauses.push("l.shore = ?");
    binds.push(f.shore);
  }
  if (f.heating_type) {
    clauses.push("l.heating_type = ?");
    binds.push(f.heating_type);
  }
  if (f.energy_class_max) {
    const rank = ENERGY_RANK[f.energy_class_max.toUpperCase()];
    if (rank != null) {
      clauses.push(
        "(l.energy_class IS NOT NULL AND instr('ABCDEFG', UPPER(l.energy_class)) > 0 AND instr('ABCDEFG', UPPER(l.energy_class)) <= ?)",
      );
      binds.push(rank);
    }
  }
  if (f.plot_ownership) {
    clauses.push("l.plot_ownership = ?");
    binds.push(f.plot_ownership);
  }
  if (f.max_days_on_market != null) {
    clauses.push("l.first_seen >= ?");
    binds.push(now() - f.max_days_on_market * 86400);
  }
  if (f.text) {
    clauses.push("(l.address LIKE ? OR l.raw_json LIKE ? OR l.municipality LIKE ?)");
    const like = `%${f.text}%`;
    binds.push(like, like, like);
  }
  for (const kw of f.exclude ?? []) {
    if (!kw) continue;
    clauses.push("(l.address IS NULL OR l.address NOT LIKE ?) AND l.raw_json NOT LIKE ?");
    const like = `%${kw}%`;
    binds.push(like, like);
  }
  if (f.price_dropped) {
    clauses.push(
      "EXISTS (SELECT 1 FROM listing_events e WHERE e.listing_id = l.id AND e.kind = 'price_change' " +
        "AND e.new_price_eur IS NOT NULL AND e.old_price_eur IS NOT NULL AND e.new_price_eur < e.old_price_eur)",
    );
  }

  return { where: clauses.length ? `WHERE ${clauses.join(" AND ")}` : "", binds };
}

function orderClause(sort: string | undefined, order: "asc" | "desc" | undefined): string {
  const col = SORTABLE[sort ?? "price"] ?? SORTABLE["price"];
  const dir = order === "desc" ? "DESC" : "ASC";
  return `ORDER BY (${col}) IS NULL, (${col}) ${dir}, l.id ASC`;
}

export interface ListingsPage {
  listings: ListingRow[];
  total: number;
}

export async function queryListings(db: D1Database, f: ListingsFilter): Promise<ListingsPage> {
  const { where, binds } = buildListingsWhere(f);
  const limit = Math.min(Math.max(f.limit ?? 50, 1), 500);
  const offset = Math.max(f.offset ?? 0, 0);

  const base = `FROM listings l LEFT JOIN listing_scores sc ON sc.listing_id = l.id ${where}`;
  const totalRow = await db
    .prepare(`SELECT COUNT(*) AS n FROM listings l ${where}`)
    .bind(...binds)
    .first<{ n: number }>();

  const { results } = await db
    .prepare(`SELECT l.* ${base} ${orderClause(f.sort, f.order)} LIMIT ? OFFSET ?`)
    .bind(...binds, limit, offset)
    .all<ListingRow>();

  return { listings: results, total: totalRow?.n ?? 0 };
}

export async function getListing(db: D1Database, id: number): Promise<ListingRow | null> {
  return db.prepare("SELECT * FROM listings WHERE id = ?").bind(id).first<ListingRow>();
}

export async function getListingEvents(db: D1Database, id: number): Promise<unknown[]> {
  const { results } = await db
    .prepare("SELECT * FROM listing_events WHERE listing_id = ? ORDER BY observed_at ASC, id ASC")
    .bind(id)
    .all();
  return results;
}

export async function getListingPhotos(db: D1Database, id: number): Promise<unknown[]> {
  const { results } = await db
    .prepare("SELECT * FROM listing_photos WHERE listing_id = ? ORDER BY position ASC")
    .bind(id)
    .all();
  return results;
}

export async function getCostInputs(db: D1Database, id: number): Promise<unknown> {
  const row = await db
    .prepare("SELECT inputs_json FROM listing_cost_inputs WHERE listing_id = ?")
    .bind(id)
    .first<{ inputs_json: string }>();
  if (!row) return null;
  return parseJson(row.inputs_json);
}

export async function setCostInputs(db: D1Database, id: number, inputs: unknown): Promise<void> {
  await db
    .prepare(
      "INSERT INTO listing_cost_inputs (listing_id, inputs_json, updated_at) VALUES (?, ?, ?) " +
        "ON CONFLICT(listing_id) DO UPDATE SET inputs_json = excluded.inputs_json, updated_at = excluded.updated_at",
    )
    .bind(id, JSON.stringify(inputs ?? {}), now())
    .run();
}

export async function setNote(db: D1Database, id: number, note: string): Promise<void> {
  await db
    .prepare(
      "INSERT INTO listing_notes (listing_id, note, updated_at) VALUES (?, ?, ?) " +
        "ON CONFLICT(listing_id) DO UPDATE SET note = excluded.note, updated_at = excluded.updated_at",
    )
    .bind(id, note, now())
    .run();
}

export async function setScore(
  db: D1Database,
  id: number,
  score: number | null,
  dealBreaker: boolean,
  criteria?: unknown,
): Promise<void> {
  await db
    .prepare(
      "INSERT INTO listing_scores (listing_id, score, criteria_json, deal_breaker, updated_at) VALUES (?, ?, ?, ?, ?) " +
        "ON CONFLICT(listing_id) DO UPDATE SET score = excluded.score, criteria_json = excluded.criteria_json, " +
        "deal_breaker = excluded.deal_breaker, updated_at = excluded.updated_at",
    )
    .bind(id, score, criteria == null ? null : JSON.stringify(criteria), dealBreaker ? 1 : 0, now())
    .run();
}

export async function setTags(db: D1Database, id: number, tags: string[]): Promise<void> {
  const statements: D1PreparedStatement[] = [
    db.prepare("DELETE FROM listing_tags WHERE listing_id = ?").bind(id),
  ];
  for (const tag of new Set(tags.filter((t) => t && t.trim() !== ""))) {
    statements.push(
      db.prepare("INSERT INTO listing_tags (listing_id, tag) VALUES (?, ?)").bind(id, tag.trim()),
    );
  }
  await db.batch(statements);
}

export async function getScore(db: D1Database, id: number): Promise<unknown> {
  return db.prepare("SELECT * FROM listing_scores WHERE listing_id = ?").bind(id).first();
}

export async function getTags(db: D1Database, id: number): Promise<string[]> {
  const { results } = await db
    .prepare("SELECT tag FROM listing_tags WHERE listing_id = ? ORDER BY tag")
    .bind(id)
    .all<{ tag: string }>();
  return results.map((r) => r.tag);
}

export async function getNote(db: D1Database, id: number): Promise<string | null> {
  const row = await db
    .prepare("SELECT note FROM listing_notes WHERE listing_id = ?")
    .bind(id)
    .first<{ note: string | null }>();
  return row?.note ?? null;
}

const LISTING_COLUMNS = [
  "property_id",
  "portal",
  "portal_listing_id",
  "url",
  "property_type",
  "holding_form",
  "kiinteistotunnus",
  "address",
  "municipality",
  "postal_code",
  "district",
  "lat",
  "lon",
  "price_eur",
  "debt_free_price_eur",
  "debt_share_eur",
  "price_per_m2",
  "maintenance_charge_eur",
  "financing_charge_eur",
  "ground_rent_eur_yr",
  "living_area_m2",
  "total_area_m2",
  "plot_area_m2",
  "room_count",
  "room_layout",
  "floors",
  "year_built",
  "occupancy_year",
  "condition_class",
  "inspection_status",
  "frame_material",
  "facade_material",
  "roof_material",
  "energy_class",
  "e_value",
  "risk_structures",
  "plot_ownership",
  "lease_end_year",
  "shore",
  "shore_sauna",
  "heating_type",
  "heat_distribution",
  "water_supply",
  "sewer_system",
  "broadband",
  "sauna",
  "parking",
  "road_access",
  "intended_use",
  "zoning_status",
  "status",
  "raw_json",
  "content_hash",
  "first_seen",
  "last_seen",
] as const;

function listingValues(n: NormalizedListing, propertyId: number | null, hash: string, ts: number): unknown[] {
  return [
    propertyId,
    n.portal,
    n.portal_listing_id,
    n.url,
    n.property_type,
    n.holding_form,
    n.kiinteistotunnus,
    n.address,
    n.municipality,
    n.postal_code,
    n.district,
    n.lat,
    n.lon,
    n.price_eur,
    n.debt_free_price_eur,
    n.debt_share_eur,
    derivePpm2(n),
    n.maintenance_charge_eur,
    n.financing_charge_eur,
    n.ground_rent_eur_yr,
    n.living_area_m2,
    n.total_area_m2,
    n.plot_area_m2,
    n.room_count,
    n.room_layout,
    n.floors,
    n.year_built,
    n.occupancy_year,
    n.condition_class,
    n.inspection_status,
    n.frame_material,
    n.facade_material,
    n.roof_material,
    n.energy_class,
    n.e_value,
    JSON.stringify(n.risk_structures ?? []),
    n.plot_ownership,
    n.lease_end_year,
    n.shore,
    n.shore_sauna,
    n.heating_type,
    n.heat_distribution,
    n.water_supply,
    n.sewer_system,
    n.broadband,
    n.sauna,
    n.parking,
    n.road_access,
    n.intended_use,
    n.zoning_status,
    n.status,
    n.raw_json,
    hash,
    ts,
    ts,
  ];
}

function derivePpm2(n: NormalizedListing): number | null {
  if (n.price_per_m2 != null) return n.price_per_m2;
  if (n.price_eur != null && n.living_area_m2 != null && n.living_area_m2 > 0) {
    return Math.round((n.price_eur / n.living_area_m2) * 100) / 100;
  }
  return null;
}

export interface UpsertResult {
  listingId: number;
  inserted: boolean;
  oldPrice: number | null;
  newPrice: number | null;
  oldStatus: string | null;
  newStatus: string;
  changed: boolean;
}

/**
 * Upsert a property by fingerprint (cross-portal dedup) returning its id. Keeps
 * first_seen, advances last_seen, backfills lat/lon when newly available.
 */
export async function upsertProperty(db: D1Database, n: NormalizedListing): Promise<number> {
  const fp = fingerprintFor(n);
  const ts = now();
  const { street, houseNo } = splitStreet(n.address);
  await db
    .prepare(
      "INSERT INTO properties (fingerprint, postal_code, municipality, street, house_no, lat, lon, first_seen, last_seen) " +
        "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) " +
        "ON CONFLICT(fingerprint) DO UPDATE SET last_seen = excluded.last_seen, " +
        "lat = COALESCE(properties.lat, excluded.lat), lon = COALESCE(properties.lon, excluded.lon), " +
        "municipality = COALESCE(properties.municipality, excluded.municipality)",
    )
    .bind(fp, n.postal_code, n.municipality, street, houseNo, n.lat, n.lon, ts, ts)
    .run();
  const row = await db
    .prepare("SELECT id FROM properties WHERE fingerprint = ?")
    .bind(fp)
    .first<{ id: number }>();
  return row?.id ?? 0;
}

/** Insert or update one listing, linking to its property, returning diff metadata. */
export async function upsertListing(db: D1Database, n: NormalizedListing): Promise<UpsertResult> {
  const ts = now();
  const hash = contentHash(n);
  const existing = await db
    .prepare("SELECT id, price_eur, status, first_seen FROM listings WHERE portal = ? AND portal_listing_id = ?")
    .bind(n.portal, n.portal_listing_id)
    .first<{ id: number; price_eur: number | null; status: string; first_seen: number }>();

  const propertyId = await upsertProperty(db, n);

  if (!existing) {
    const cols = LISTING_COLUMNS.join(", ");
    const placeholders = LISTING_COLUMNS.map(() => "?").join(", ");
    const res = await db
      .prepare(`INSERT INTO listings (${cols}) VALUES (${placeholders})`)
      .bind(...listingValues(n, propertyId, hash, ts))
      .run();
    const listingId = Number(res.meta.last_row_id);
    return {
      listingId,
      inserted: true,
      oldPrice: null,
      newPrice: n.price_eur,
      oldStatus: null,
      newStatus: n.status,
      changed: true,
    };
  }

  const changed = existing.price_eur !== n.price_eur || existing.status !== n.status;
  await db
    .prepare(
      "UPDATE listings SET property_id = ?, url = ?, property_type = ?, holding_form = ?, kiinteistotunnus = ?, " +
        "address = ?, municipality = ?, postal_code = ?, district = ?, lat = ?, lon = ?, price_eur = ?, " +
        "debt_free_price_eur = ?, debt_share_eur = ?, price_per_m2 = ?, maintenance_charge_eur = ?, " +
        "financing_charge_eur = ?, ground_rent_eur_yr = ?, living_area_m2 = ?, total_area_m2 = ?, plot_area_m2 = ?, " +
        "room_count = ?, room_layout = ?, floors = ?, year_built = ?, occupancy_year = ?, condition_class = ?, " +
        "inspection_status = ?, frame_material = ?, facade_material = ?, roof_material = ?, energy_class = ?, " +
        "e_value = ?, risk_structures = ?, plot_ownership = ?, lease_end_year = ?, shore = ?, shore_sauna = ?, " +
        "heating_type = ?, heat_distribution = ?, water_supply = ?, sewer_system = ?, broadband = ?, sauna = ?, " +
        "parking = ?, road_access = ?, intended_use = ?, zoning_status = ?, status = ?, raw_json = ?, " +
        "content_hash = ?, last_seen = ? WHERE id = ?",
    )
    .bind(
      propertyId,
      n.url,
      n.property_type,
      n.holding_form,
      n.kiinteistotunnus,
      n.address,
      n.municipality,
      n.postal_code,
      n.district,
      n.lat,
      n.lon,
      n.price_eur,
      n.debt_free_price_eur,
      n.debt_share_eur,
      derivePpm2(n),
      n.maintenance_charge_eur,
      n.financing_charge_eur,
      n.ground_rent_eur_yr,
      n.living_area_m2,
      n.total_area_m2,
      n.plot_area_m2,
      n.room_count,
      n.room_layout,
      n.floors,
      n.year_built,
      n.occupancy_year,
      n.condition_class,
      n.inspection_status,
      n.frame_material,
      n.facade_material,
      n.roof_material,
      n.energy_class,
      n.e_value,
      JSON.stringify(n.risk_structures ?? []),
      n.plot_ownership,
      n.lease_end_year,
      n.shore,
      n.shore_sauna,
      n.heating_type,
      n.heat_distribution,
      n.water_supply,
      n.sewer_system,
      n.broadband,
      n.sauna,
      n.parking,
      n.road_access,
      n.intended_use,
      n.zoning_status,
      n.status,
      n.raw_json,
      hash,
      ts,
      existing.id,
    )
    .run();

  return {
    listingId: existing.id,
    inserted: false,
    oldPrice: existing.price_eur,
    newPrice: n.price_eur,
    oldStatus: existing.status,
    newStatus: n.status,
    changed,
  };
}

export async function appendEvent(
  db: D1Database,
  listingId: number,
  kind: string,
  fields: { oldPrice?: number | null; newPrice?: number | null; oldValue?: string | null; newValue?: string | null },
): Promise<void> {
  await db
    .prepare(
      "INSERT INTO listing_events (listing_id, kind, old_price_eur, new_price_eur, old_value, new_value, observed_at) " +
        "VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(
      listingId,
      kind,
      fields.oldPrice ?? null,
      fields.newPrice ?? null,
      fields.oldValue ?? null,
      fields.newValue ?? null,
      now(),
    )
    .run();
}

/** Record diff events for an upsert result (first_seen, price_change, status_change). */
export async function recordDiffEvents(db: D1Database, r: UpsertResult): Promise<void> {
  if (r.inserted) {
    await appendEvent(db, r.listingId, "first_seen", { newPrice: r.newPrice });
    return;
  }
  if (r.oldPrice !== r.newPrice) {
    await appendEvent(db, r.listingId, "price_change", { oldPrice: r.oldPrice, newPrice: r.newPrice });
  }
  if (r.oldStatus !== r.newStatus) {
    const kind = r.newStatus === "active" && r.oldStatus !== "active" ? "relisted" : "status_change";
    await appendEvent(db, r.listingId, kind, { oldValue: r.oldStatus, newValue: r.newStatus });
  }
}

export async function isPhotoSeen(db: D1Database, url: string): Promise<string | null> {
  const row = await db.prepare("SELECT r2_key FROM seen_photo_urls WHERE url = ?").bind(url).first<{
    r2_key: string;
  }>();
  return row?.r2_key ?? null;
}

export async function recordPhoto(
  db: D1Database,
  listingId: number,
  position: number,
  r2Key: string,
  sourceUrl: string,
  contentType: string | null,
): Promise<void> {
  await db.batch([
    db
      .prepare(
        "INSERT INTO listing_photos (listing_id, position, r2_key, content_type, source_url) VALUES (?, ?, ?, ?, ?) " +
          "ON CONFLICT(listing_id, position) DO UPDATE SET r2_key = excluded.r2_key, content_type = excluded.content_type, source_url = excluded.source_url",
      )
      .bind(listingId, position, r2Key, contentType, sourceUrl),
    db
      .prepare("INSERT INTO seen_photo_urls (url, r2_key) VALUES (?, ?) ON CONFLICT(url) DO NOTHING")
      .bind(sourceUrl, r2Key),
  ]);
}

export async function getCrawlState(db: D1Database, source: string): Promise<CrawlStateRow | null> {
  return db.prepare("SELECT * FROM crawl_state WHERE source = ?").bind(source).first<CrawlStateRow>();
}

export async function listCrawlState(db: D1Database): Promise<CrawlStateRow[]> {
  const { results } = await db
    .prepare("SELECT * FROM crawl_state ORDER BY source")
    .all<CrawlStateRow>();
  return results;
}

export async function pickNextSource(db: D1Database): Promise<CrawlStateRow | null> {
  const running = await db
    .prepare("SELECT * FROM crawl_state WHERE status = 'running' ORDER BY last_tick ASC LIMIT 1")
    .first<CrawlStateRow>();
  if (running) return running;
  return db
    .prepare(
      "SELECT * FROM crawl_state WHERE status IN ('idle','done','error') ORDER BY COALESCE(last_tick, 0) ASC LIMIT 1",
    )
    .first<CrawlStateRow>();
}

export async function ensureCrawlSource(db: D1Database, source: string): Promise<void> {
  await db
    .prepare(
      "INSERT INTO crawl_state (source, updated_at) VALUES (?, ?) ON CONFLICT(source) DO NOTHING",
    )
    .bind(source, now())
    .run();
}

export async function advanceCrawlState(
  db: D1Database,
  source: string,
  fields: Partial<Pick<CrawlStateRow, "next_page" | "total_pages" | "cursor" | "status" | "last_error">>,
): Promise<void> {
  await db
    .prepare(
      "UPDATE crawl_state SET next_page = COALESCE(?, next_page), total_pages = COALESCE(?, total_pages), " +
        "cursor = ?, status = COALESCE(?, status), last_error = ?, last_tick = ?, updated_at = ? WHERE source = ?",
    )
    .bind(
      fields.next_page ?? null,
      fields.total_pages ?? null,
      fields.cursor ?? null,
      fields.status ?? null,
      fields.last_error ?? null,
      now(),
      now(),
      source,
    )
    .run();
}

export async function listSavedSearches(db: D1Database): Promise<SavedSearchRow[]> {
  const { results } = await db
    .prepare("SELECT * FROM saved_searches ORDER BY created_at DESC")
    .all<SavedSearchRow>();
  return results;
}

export async function getSavedSearch(db: D1Database, id: number): Promise<SavedSearchRow | null> {
  return db.prepare("SELECT * FROM saved_searches WHERE id = ?").bind(id).first<SavedSearchRow>();
}

export async function createSavedSearch(
  db: D1Database,
  name: string,
  params: unknown,
  isExact: boolean,
): Promise<SavedSearchRow> {
  const res = await db
    .prepare("INSERT INTO saved_searches (name, params_json, is_exact, created_at) VALUES (?, ?, ?, ?)")
    .bind(name, JSON.stringify(params ?? {}), isExact ? 1 : 0, now())
    .run();
  const id = Number(res.meta.last_row_id);
  return (await getSavedSearch(db, id))!;
}

export async function updateSavedSearch(
  db: D1Database,
  id: number,
  fields: { name?: string; params?: unknown; isExact?: boolean },
): Promise<SavedSearchRow | null> {
  const existing = await getSavedSearch(db, id);
  if (!existing) return null;
  await db
    .prepare("UPDATE saved_searches SET name = ?, params_json = ?, is_exact = ? WHERE id = ?")
    .bind(
      fields.name ?? existing.name,
      fields.params !== undefined ? JSON.stringify(fields.params ?? {}) : existing.params_json,
      fields.isExact !== undefined ? (fields.isExact ? 1 : 0) : existing.is_exact,
      id,
    )
    .run();
  return getSavedSearch(db, id);
}

export async function deleteSavedSearch(db: D1Database, id: number): Promise<boolean> {
  const res = await db.prepare("DELETE FROM saved_searches WHERE id = ?").bind(id).run();
  return (res.meta.changes ?? 0) > 0;
}

export async function getDossier(db: D1Database, propertyId: number): Promise<unknown> {
  const row = await db
    .prepare("SELECT dossier_json, enriched_at FROM location_dossier WHERE property_id = ?")
    .bind(propertyId)
    .first<{ dossier_json: string; enriched_at: number }>();
  if (!row) return null;
  return { ...(parseJson(row.dossier_json) as object), enriched_at: row.enriched_at };
}

export async function setDossier(db: D1Database, propertyId: number, dossier: unknown): Promise<void> {
  await db
    .prepare(
      "INSERT INTO location_dossier (property_id, dossier_json, enriched_at) VALUES (?, ?, ?) " +
        "ON CONFLICT(property_id) DO UPDATE SET dossier_json = excluded.dossier_json, enriched_at = excluded.enriched_at",
    )
    .bind(propertyId, JSON.stringify(dossier ?? {}), now())
    .run();
}

export async function getProperty(db: D1Database, id: number): Promise<Record<string, unknown> | null> {
  return db
    .prepare("SELECT * FROM properties WHERE id = ?")
    .bind(id)
    .first<Record<string, unknown>>();
}

export async function getListingsForProperty(db: D1Database, propertyId: number): Promise<ListingRow[]> {
  const { results } = await db
    .prepare("SELECT * FROM listings WHERE property_id = ? ORDER BY last_seen DESC")
    .bind(propertyId)
    .all<ListingRow>();
  return results;
}

export async function getMarketStats(db: D1Database, municipality: string): Promise<unknown[]> {
  const { results } = await db
    .prepare(
      "SELECT * FROM market_stats WHERE area_kind = 'municipality' AND area_code = ? COLLATE NOCASE ORDER BY fetched_at DESC",
    )
    .bind(municipality)
    .all();
  return results;
}

export async function setMarketStat(
  db: D1Database,
  stat: {
    area_kind: string;
    area_code: string;
    metric: string;
    property_kind: string | null;
    period: string | null;
    value: number;
    source: string;
  },
): Promise<void> {
  await db
    .prepare(
      "INSERT INTO market_stats (area_kind, area_code, metric, property_kind, period, value, source, fetched_at) " +
        "VALUES (?, ?, ?, ?, ?, ?, ?, ?) " +
        "ON CONFLICT(area_kind, area_code, metric, property_kind, period, source) DO UPDATE SET " +
        "value = excluded.value, fetched_at = excluded.fetched_at",
    )
    .bind(
      stat.area_kind,
      stat.area_code,
      stat.metric,
      stat.property_kind,
      stat.period,
      stat.value,
      stat.source,
      now(),
    )
    .run();
}

export async function listPropertiesNeedingEnrichment(db: D1Database, limit: number): Promise<
  Array<{ id: number; lat: number | null; lon: number | null; municipality: string | null; postal_code: string | null; street: string | null; house_no: string | null }>
> {
  const { results } = await db
    .prepare(
      "SELECT p.id, p.lat, p.lon, p.municipality, p.postal_code, p.street, p.house_no FROM properties p " +
        "LEFT JOIN location_dossier d ON d.property_id = p.id WHERE d.property_id IS NULL ORDER BY p.last_seen DESC LIMIT ?",
    )
    .bind(limit)
    .all<{ id: number; lat: number | null; lon: number | null; municipality: string | null; postal_code: string | null; street: string | null; house_no: string | null }>();
  return results;
}

function splitStreet(address: string | null): { street: string | null; houseNo: string | null } {
  if (!address) return { street: null, houseNo: null };
  const m = address.match(/^(.*?)(\d+[a-zA-Z]?)\s*$/);
  if (m) return { street: (m[1] ?? "").trim() || null, houseNo: (m[2] ?? "").trim() || null };
  return { street: address.trim() || null, houseNo: null };
}

function parseJson(s: string): unknown {
  try {
    return JSON.parse(s);
  } catch {
    return null;
  }
}
