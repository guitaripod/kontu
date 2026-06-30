/**
 * Plane-B location enrichment. Background-only, fully resilient: every external
 * call is wrapped, returns graceful nulls, and never throws or blocks an API
 * request. Overpass nearest-services is the openly-reachable backbone; the other
 * sources (SYKE water/flood, Traficom broadband, Digitransit travel) are stubbed
 * behind named functions that return null when unreachable, so partial dossiers
 * still persist. Results cache to `location_dossier`.
 */
import { listPropertiesNeedingEnrichment, setDossier } from "./db";

export interface NearestServices {
  grocery: ServiceHit | null;
  school: ServiceHit | null;
  health: ServiceHit | null;
  town: ServiceHit | null;
}

export interface ServiceHit {
  name: string | null;
  distance_m: number;
  lat: number;
  lon: number;
}

export interface LocationDossier {
  lat: number | null;
  lon: number | null;
  distance_to_water_m: number | null;
  nearest_services: NearestServices;
  broadband: BroadbandInfo | null;
  flood_risk: FloodInfo | null;
  travel_times: TravelTimes | null;
  partial: boolean;
}

export interface BroadbandInfo {
  fibre: boolean | null;
  min_100mbit: boolean | null;
  min_1gbit: boolean | null;
}

export interface FloodInfo {
  in_zone: boolean | null;
  depth_class: string | null;
  return_period_years: number | null;
}

export interface TravelTimes {
  car_to_town_min: number | null;
  transit_to_town_min: number | null;
}

// Several public Overpass mirrors. Any one rate-limits a busy egress IP (and stays
// blocked for hours), so a query that fails on one falls through to the next.
const OVERPASS_URLS = [
  "https://overpass-api.de/api/interpreter",
  "https://maps.mail.ru/osm/tools/overpass/api/interpreter",
  "https://overpass.osm.ch/api/interpreter",
];

/** POST an Overpass query across the mirrors; the first 2xx response wins, else null. */
async function overpassFetch(body: string): Promise<Response | null> {
  for (const url of OVERPASS_URLS) {
    try {
      const res = await fetch(url, {
        method: "POST",
        headers: { "Content-Type": "text/plain", Accept: "application/json" },
        body,
      });
      if (res.ok) return res;
    } catch {
      /* mirror unreachable — try the next */
    }
  }
  return null;
}
const PELIAS_URL =
  "https://avoin-paikkatieto.maanmittauslaitos.fi/geocoding/v2/pelias/search";

interface GeoInput {
  lat: number | null;
  lon: number | null;
  municipality: string | null;
  postal_code: string | null;
  street: string | null;
  house_no: string | null;
}

/**
 * Build a (possibly partial) dossier for one property. Geocodes via MML Pelias
 * when lat/lon are missing, then enriches. Always resolves to a dossier object.
 */
export async function buildDossier(input: GeoInput): Promise<LocationDossier> {
  let lat = input.lat;
  let lon = input.lon;
  if (lat == null || lon == null) {
    const geo = await geocode(input);
    if (geo) {
      lat = geo.lat;
      lon = geo.lon;
    }
  }

  const nearest_services: NearestServices = { grocery: null, school: null, health: null, town: null };
  let distance_to_water_m: number | null = null;
  let broadband: BroadbandInfo | null = null;
  let flood_risk: FloodInfo | null = null;
  let travel_times: TravelTimes | null = null;

  if (lat != null && lon != null) {
    const services = await nearestServices(lat, lon);
    Object.assign(nearest_services, services);
    distance_to_water_m = await distanceToWater(lat, lon);
    broadband = await broadbandAvailability(lat, lon);
    flood_risk = await floodRisk(lat, lon);
    travel_times = await travelTimes(lat, lon);
  }

  const partial =
    lat == null ||
    lon == null ||
    distance_to_water_m == null ||
    broadband == null ||
    flood_risk == null;

  return { lat, lon, distance_to_water_m, nearest_services, broadband, flood_risk, travel_times, partial };
}

/** Enrich up to `limit` un-enriched properties; cache each dossier. Never throws. */
export async function enrichBatch(db: D1Database, limit: number): Promise<number> {
  let done = 0;
  try {
    const targets = await listPropertiesNeedingEnrichment(db, limit);
    for (const t of targets) {
      try {
        const dossier = await buildDossier(t);
        await setDossier(db, t.id, dossier);
        done++;
      } catch (err) {
        console.warn("enrich property failed", t.id, String(err));
      }
    }
  } catch (err) {
    console.warn("enrich batch failed", String(err));
  }
  return done;
}

/**
 * Automated geometric shore detection for ANY country — the country-agnostic
 * analogue of Finland's SYKE water layer, via OSM. From a listing's coordinates:
 * a water body within ~150 m → own/near shore (lake `jarvi`, or sea `meri` near a
 * coastline), else `ei_rantaa`. Runs a small batch per scheduled tick so every
 * listing eventually gets a shore signal without a portal-provided field. This is
 * what lets a Swedish/Norwegian lakeside match be DETECTED automatically. FI keeps
 * its portal shore (only NULL shores are filled).
 */
export async function enrichShoreBatch(db: D1Database, limit: number): Promise<number> {
  let done = 0;
  try {
    const { results } = await db
      .prepare(
        "SELECT id, lat, lon FROM listings WHERE shore IS NULL AND lat IS NOT NULL AND lon IS NOT NULL " +
          "AND status = 'active' ORDER BY last_seen DESC LIMIT ?",
      )
      .bind(limit)
      .all<{ id: number; lat: number; lon: number }>();
    for (const r of results) {
      const probe = await shoreFromOsm(r.lat, r.lon);
      if (!probe.ok) continue; // query failed — leave shore NULL so it's retried, not poisoned
      await db
        .prepare("UPDATE listings SET shore = ?, water_body = ? WHERE id = ?")
        .bind(probe.water == null ? "ei_rantaa" : "oma_ranta", probe.water, r.id)
        .run();
      done++;
      await new Promise((res) => setTimeout(res, 1100)); // be gentle on the public Overpass API
    }
  } catch (err) {
    console.warn("shore enrichment failed", String(err));
  }
  return done;
}

/** Result of a geometric shore probe. `ok: false` means the Overpass query FAILED
 *  (HTTP error / rate-limit / timeout) — distinct from a successful query that found
 *  no water (`ok: true, water: null`). The caller must NOT mark a failed probe as
 *  "no shore", or a transient Overpass blip permanently poisons the listing. */
type ShoreProbe = { ok: false } | { ok: true; water: "jarvi" | "meri" | null };

/** Lake within ~150 m → 'jarvi'; coastline within ~400 m → 'meri'; else no water. */
async function shoreFromOsm(lat: number, lon: number): Promise<ShoreProbe> {
  try {
    const q =
      `[out:json][timeout:20];(` +
      `way["natural"="water"](around:150,${lat},${lon});` +
      `relation["natural"="water"](around:150,${lat},${lon});` +
      `way["natural"="coastline"](around:400,${lat},${lon}););out tags 1;`;
    const res = await overpassFetch(q);
    if (!res) return { ok: false };
    const body = (await res.json()) as { elements?: Array<{ tags?: Record<string, string> }>; remark?: string };
    // Overpass signals a soft failure (timeout / out-of-memory) with a `remark` and an
    // empty/absent `elements` — that is NOT "no water nearby". Treat it as a failed probe
    // (leave the shore pending) instead of poisoning the listing as shoreless.
    if (body.elements == null || (body.remark != null && (body.elements?.length ?? 0) === 0)) {
      return { ok: false };
    }
    const els = body.elements;
    const coast = els.some((e) => e.tags?.natural === "coastline");
    const firstWater = els.find((e) => e.tags?.natural === "water");
    if (firstWater) {
      const t = firstWater.tags ?? {};
      return { ok: true, water: t.water === "bay" || t.water === "lagoon" || coast ? "meri" : "jarvi" };
    }
    return { ok: true, water: coast ? "meri" : null };
  } catch {
    return { ok: false };
  }
}

async function geocode(input: GeoInput): Promise<{ lat: number; lon: number } | null> {
  const text = [input.street, input.house_no, input.postal_code, input.municipality]
    .filter((s) => s != null && String(s).trim() !== "")
    .join(" ");
  if (text === "") return null;
  try {
    const url = `${PELIAS_URL}?text=${encodeURIComponent(text)}&size=1&lang=fi`;
    const res = await fetch(url, { headers: { Accept: "application/json" } });
    if (!res.ok) return null;
    const body = (await res.json()) as { features?: Array<{ geometry?: { coordinates?: number[] } }> };
    const coords = body.features?.[0]?.geometry?.coordinates;
    if (coords && coords.length >= 2 && typeof coords[0] === "number" && typeof coords[1] === "number") {
      return { lon: coords[0], lat: coords[1] };
    }
    return null;
  } catch (err) {
    console.warn("geocode failed", String(err));
    return null;
  }
}

const SERVICE_QUERIES: Array<{ key: keyof NearestServices; filter: string; radius: number }> = [
  { key: "grocery", filter: 'node["shop"~"supermarket|convenience"]', radius: 15000 },
  { key: "school", filter: 'node["amenity"="school"]', radius: 20000 },
  { key: "health", filter: 'node["amenity"~"hospital|clinic|doctors|pharmacy"]', radius: 25000 },
  { key: "town", filter: 'node["place"~"town|city"]', radius: 40000 },
];

async function nearestServices(lat: number, lon: number): Promise<Partial<NearestServices>> {
  const out: Partial<NearestServices> = {};
  const blocks = SERVICE_QUERIES.map(
    (q) => `${q.filter}(around:${q.radius},${lat},${lon});`,
  ).join("\n");
  const query = `[out:json][timeout:25];(${blocks});out body center;`;
  try {
    const res = await overpassFetch(query);
    if (!res) return out;
    const body = (await res.json()) as { elements?: OverpassElement[] };
    const elements = body.elements ?? [];
    for (const q of SERVICE_QUERIES) {
      out[q.key] = nearestOfKind(lat, lon, elements, q);
    }
    return out;
  } catch (err) {
    console.warn("overpass nearest services failed", String(err));
    return out;
  }
}

interface OverpassElement {
  type: string;
  lat?: number;
  lon?: number;
  center?: { lat: number; lon: number };
  tags?: Record<string, string>;
}

function nearestOfKind(
  lat: number,
  lon: number,
  elements: OverpassElement[],
  q: { key: keyof NearestServices; filter: string },
): ServiceHit | null {
  let best: ServiceHit | null = null;
  const wantShop = q.key === "grocery";
  const wantPlace = q.key === "town";
  for (const el of elements) {
    const elat = el.lat ?? el.center?.lat;
    const elon = el.lon ?? el.center?.lon;
    if (elat == null || elon == null) continue;
    const tags = el.tags ?? {};
    if (wantShop && !tags["shop"]) continue;
    if (wantPlace && !tags["place"]) continue;
    if (q.key === "school" && tags["amenity"] !== "school") continue;
    if (q.key === "health" && !/hospital|clinic|doctors|pharmacy/.test(tags["amenity"] ?? "")) continue;
    const d = haversineMeters(lat, lon, elat, elon);
    if (best == null || d < best.distance_m) {
      best = { name: tags["name"] ?? null, distance_m: Math.round(d), lat: elat, lon: elon };
    }
  }
  return best;
}

/** SYKE Ranta10 nearest-water lookup. Stubbed (network-gated) → null when unreachable. */
async function distanceToWater(lat: number, lon: number): Promise<number | null> {
  try {
    const query =
      `[out:json][timeout:25];(way["natural"="water"](around:5000,${lat},${lon});` +
      `relation["natural"="water"](around:5000,${lat},${lon}););out center 1;`;
    const res = await overpassFetch(query);
    if (!res) return null;
    const body = (await res.json()) as { elements?: OverpassElement[] };
    let best: number | null = null;
    for (const el of body.elements ?? []) {
      const elat = el.center?.lat ?? el.lat;
      const elon = el.center?.lon ?? el.lon;
      if (elat == null || elon == null) continue;
      const d = haversineMeters(lat, lon, elat, elon);
      if (best == null || d < best) best = d;
    }
    return best == null ? null : Math.round(best);
  } catch (err) {
    console.warn("distance to water failed", String(err));
    return null;
  }
}

/** Traficom broadband availability. Stubbed (auth/format-gated) → null. */
async function broadbandAvailability(_lat: number, _lon: number): Promise<BroadbandInfo | null> {
  return null;
}

/** SYKE flood-zone WFS. Stubbed (WFS-gated) → null. */
async function floodRisk(_lat: number, _lon: number): Promise<FloodInfo | null> {
  return null;
}

/** Digitransit Routing v2 travel times. Stubbed (subscription-key-gated) → null. */
async function travelTimes(_lat: number, _lon: number): Promise<TravelTimes | null> {
  return null;
}

function haversineMeters(lat1: number, lon1: number, lat2: number, lon2: number): number {
  const R = 6371000;
  const toRad = (d: number): number => (d * Math.PI) / 180;
  const dLat = toRad(lat2 - lat1);
  const dLon = toRad(lon2 - lon1);
  const a =
    Math.sin(dLat / 2) ** 2 +
    Math.cos(toRad(lat1)) * Math.cos(toRad(lat2)) * Math.sin(dLon / 2) ** 2;
  return 2 * R * Math.asin(Math.min(1, Math.sqrt(a)));
}

export type BugBand = "matala" | "kohtalainen" | "korkea";

export interface BugIndex {
  score: number;
  band: BugBand;
}

export interface BugPressure {
  mosquito: BugIndex;
  blackfly: BugIndex;
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
}

const SYKE_WCS = "https://paikkatiedot.ymparisto.fi/geoserver/inspire_lc/wcs";
const SYKE_WFS = "https://paikkatiedot.ymparisto.fi/geoserver/inspire_hy/wfs";
const CLC_BOX_M = 1000;
const WATERCOURSE_RADIUS_M = 2500;

/** CLC2018 (SYKE) class codes that hold standing water — mosquito breeding habitat. */
const OPEN_MIRE = new Set([41, 42, 43, 44, 45, 46]);
const PEAT_FOREST = new Set([24, 26, 29, 35]);
const LAKE = new Set([48]);
const SEA = new Set([49]);

const clamp01 = (x: number): number => Math.max(0, Math.min(1, x));

function bandOf(score: number): BugBand {
  return score < 0.09 ? "matala" : score < 0.28 ? "kohtalainen" : "korkea";
}

/// Räkkä season worsens northward; mild multiplier anchored at 60–68°N.
function latitudeFactor(lat: number): number {
  return 0.85 + 0.35 * clamp01((lat - 60) / 8);
}

/// WGS84 → ETRS-TM35FIN (EPSG:3067), the CRS SYKE's geodata is served in.
function wgs84ToTm35fin(lat: number, lon: number): { E: number; N: number } {
  const a = 6378137.0;
  const f = 1 / 298.257222101;
  const k0 = 0.9996;
  const FE = 500000.0;
  const lon0 = (27.0 * Math.PI) / 180;
  const rlat = (lat * Math.PI) / 180;
  const rlon = (lon * Math.PI) / 180;
  const n = f / (2 - f);
  const A = (a / (1 + n)) * (1 + n ** 2 / 4 + n ** 4 / 64);
  const a1 = n / 2 - (2 * n ** 2) / 3 + (5 * n ** 3) / 16;
  const a2 = (13 * n ** 2) / 48 - (3 * n ** 3) / 5;
  const a3 = (61 * n ** 3) / 240;
  const ep = Math.sqrt(f * (2 - f));
  const Q = Math.asinh(Math.tan(rlat)) - ep * Math.atanh(ep * Math.sin(rlat));
  const be = Math.atan(Math.sinh(Q));
  const eta0 = Math.atanh(Math.cos(be) * Math.sin(rlon - lon0));
  const xi0 = Math.asin(Math.sin(be) * Math.cosh(eta0));
  const xi =
    xi0 +
    a1 * Math.sin(2 * xi0) * Math.cosh(2 * eta0) +
    a2 * Math.sin(4 * xi0) * Math.cosh(4 * eta0) +
    a3 * Math.sin(6 * xi0) * Math.cosh(6 * eta0);
  const eta =
    eta0 +
    a1 * Math.cos(2 * xi0) * Math.sinh(2 * eta0) +
    a2 * Math.cos(4 * xi0) * Math.sinh(4 * eta0) +
    a3 * Math.cos(6 * xi0) * Math.sinh(6 * eta0);
  return { N: A * xi * k0, E: A * eta * k0 + FE };
}

interface ClcStats {
  open_mire: number;
  peat_forest: number;
  lake: number;
  sea: number;
}

/// Sample SYKE Corine Land Cover 2018 over a 2 km box as a plaintext class grid
/// (avoids a GeoTIFF decoder); fraction of each standing-water habitat class.
async function clcHabitat(E: number, N: number): Promise<ClcStats | null> {
  const url =
    `${SYKE_WCS}?service=WCS&version=2.0.1&request=GetCoverage` +
    `&coverageId=inspire_lc__LC.LandCoverRaster.2018` +
    `&subset=E(${E - CLC_BOX_M},${E + CLC_BOX_M})&subset=N(${N - CLC_BOX_M},${N + CLC_BOX_M})` +
    `&format=text/plain`;
  try {
    const res = await fetch(url, { signal: AbortSignal.timeout(15000) });
    if (!res.ok) return null;
    const text = await res.text();
    const lastBracket = text.lastIndexOf("]");
    const tail = lastBracket >= 0 ? text.slice(lastBracket + 1) : text;
    const cells = (tail.match(/\d+/g) || [])
      .map(Number)
      .filter((v) => v >= 0 && v <= 255 && v !== 255);
    if (cells.length < 100) return null;
    const frac = (set: Set<number>): number => cells.filter((v) => set.has(v)).length / cells.length;
    return { open_mire: frac(OPEN_MIRE), peat_forest: frac(PEAT_FOREST), lake: frac(LAKE), sea: frac(SEA) };
  } catch (err) {
    console.warn("clc habitat fetch failed", String(err));
    return null;
  }
}

/// Weighted flowing-water length within radius from SYKE's watercourse network
/// (blackfly larvae need oxygenated running water); larger channels weigh more.
async function watercourseKm(E: number, N: number): Promise<number | null> {
  const r = WATERCOURSE_RADIUS_M;
  const url =
    `${SYKE_WFS}?service=WFS&version=2.0.0&request=GetFeature` +
    `&typeNames=inspire_hy:HY.Network.WatercourseLink&outputFormat=application/json&srsName=EPSG:3067` +
    `&bbox=${E - r},${N - r},${E + r},${N + r},urn:ogc:def:crs:EPSG::3067&count=400`;
  try {
    const res = await fetch(url, { signal: AbortSignal.timeout(15000) });
    if (!res.ok) return null;
    const body = (await res.json()) as { features?: WfsFeature[] };
    let km = 0;
    for (const ft of body.features ?? []) {
      const klass = ft.properties?.uomaluokka;
      const w = klass === 1 ? 1.0 : klass === 2 ? 0.85 : 0.7;
      const g = ft.geometry;
      const lines: number[][][] =
        g?.type === "MultiLineString"
          ? (g.coordinates as number[][][])
          : g?.type === "LineString"
            ? [g.coordinates as number[][]]
            : [];
      for (const line of lines) {
        const near = line.filter((p) => p.length >= 2 && Math.hypot((p[0] as number) - E, (p[1] as number) - N) <= r);
        for (let i = 1; i < near.length; i++) {
          const a = near[i - 1] as number[];
          const b = near[i] as number[];
          km += (Math.hypot((b[0] as number) - (a[0] as number), (b[1] as number) - (a[1] as number)) / 1000) * w;
        }
      }
    }
    return km;
  } catch (err) {
    console.warn("watercourse fetch failed", String(err));
    return null;
  }
}

interface WfsFeature {
  properties?: { uomaluokka?: number };
  geometry?: { type?: string; coordinates?: unknown };
}

const MML_BASE = "https://avoin-paikkatieto.maanmittauslaitos.fi/maastotiedot/features/v1";
const MML_CRS3067 = "http://www.opengis.net/def/crs/EPSG/0/3067";

const MML_MIRE_BOX_M = 1000;
const MIRE_GRID = 24;

/// Authoritative mire/wetland coverage from MML maastotietokanta (suo + soistuma)
/// as the fraction of the same 2 km box CLC uses. Polygons (EPSG:3067 metres) are
/// box-clipped by grid-sampling point-in-polygon, so a mire merely touching the
/// box edge can't inflate the result. Returns null on failure → caller uses CLC.
async function mmlMireFraction(lat: number, lon: number, key: string): Promise<number | null> {
  const { E, N } = wgs84ToTm35fin(lat, lon);
  const r = MML_MIRE_BOX_M;
  const dLat = r / 111320;
  const dLon = r / (111320 * Math.cos((lat * Math.PI) / 180));
  const bbox = `${lon - dLon},${lat - dLat},${lon + dLon},${lat + dLat}`;
  const polys: number[][][][] = [];
  let reached = false;
  for (const coll of ["suo", "soistuma"]) {
    const url =
      `${MML_BASE}/collections/${coll}/items?bbox=${bbox}` +
      `&crs=${encodeURIComponent(MML_CRS3067)}&f=json&limit=2000`;
    try {
      // Key via HTTP Basic auth (username = key) so it never sits in a URL that
      // could leak into an error log.
      const res = await fetch(url, {
        headers: { Authorization: `Basic ${btoa(`${key}:`)}` },
        signal: AbortSignal.timeout(15000),
      });
      if (!res.ok) continue;
      const body = (await res.json()) as { features?: { geometry?: { type?: string; coordinates?: unknown } }[] };
      reached = true;
      for (const f of body.features ?? []) {
        const g = f.geometry;
        if (g?.type === "Polygon") polys.push(g.coordinates as number[][][]);
        else if (g?.type === "MultiPolygon") for (const p of (g.coordinates as number[][][][]) ?? []) polys.push(p);
      }
    } catch (err) {
      console.warn("mml mire fetch failed", coll, String(err));
    }
  }
  if (!reached) return null;
  if (polys.length === 0) return 0;
  let inside = 0;
  let total = 0;
  for (let i = 0; i <= MIRE_GRID; i++) {
    for (let j = 0; j <= MIRE_GRID; j++) {
      const x = E - r + (2 * r * i) / MIRE_GRID;
      const y = N - r + (2 * r * j) / MIRE_GRID;
      total++;
      if (pointInAnyMire(polys, x, y)) inside++;
    }
  }
  return inside / total;
}

function pointInAnyMire(polys: number[][][][], x: number, y: number): boolean {
  for (const rings of polys) {
    if (rings.length === 0 || !pointInRing(rings[0] as number[][], x, y)) continue;
    let inHole = false;
    for (let h = 1; h < rings.length; h++) {
      if (pointInRing(rings[h] as number[][], x, y)) {
        inHole = true;
        break;
      }
    }
    if (!inHole) return true;
  }
  return false;
}

/// Ray-casting point-in-ring; coords [x,y] in metres.
function pointInRing(ring: number[][], x: number, y: number): boolean {
  let inside = false;
  for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
    const pi = ring[i] as number[];
    const pj = ring[j] as number[];
    const xi = pi[0] as number;
    const yi = pi[1] as number;
    const xj = pj[0] as number;
    const yj = pj[1] as number;
    if (yi > y !== yj > y && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) inside = !inside;
  }
  return inside;
}

/**
 * Soft, informational bug-pressure for a point, from open Finnish geodata:
 * mosquitoes (hyttyset) from standing water (SYKE CLC mires/wetlands/lakes),
 * blackflies (mäkärät) from flowing water (SYKE watercourse network), with a
 * mild northward latitude factor. Never gates a listing. Resilient → null on
 * total failure; `partial` when one source was unreachable.
 */
export async function bugPressure(lat: number, lon: number, mmlKey?: string): Promise<BugPressure | null> {
  const { E, N } = wgs84ToTm35fin(lat, lon);
  const [clc, water, mmlMire] = await Promise.all([
    clcHabitat(E, N),
    watercourseKm(E, N),
    mmlKey ? mmlMireFraction(lat, lon, mmlKey) : Promise.resolve(null),
  ]);
  if (clc == null && water == null && mmlMire == null) return null;
  const lf = latitudeFactor(lat);

  // Mire/wetland fraction: prefer authoritative MML suo polygons; else the coarser
  // CLC proxy (open mire + weighted peatland forest).
  const mireFrac = mmlMire != null ? mmlMire : clc ? clc.open_mire + clc.peat_forest * 0.45 : 0;
  const lake = clc ? clc.lake : 0;
  const sea = clc ? clc.sea : 0;
  const usingMml = mmlMire != null;

  const habitat = mireFrac * 1.0 + lake * 0.3 + sea * 0.15;
  const mosquitoScore = clamp01(habitat) * lf;
  const blackflyScore = clamp01((water ?? 0) / 3.0) * lf;
  const pct = (x: number): number => Math.round(x * 1000) / 10;

  return {
    mosquito: { score: Math.round(mosquitoScore * 100) / 100, band: bandOf(mosquitoScore) },
    blackfly: { score: Math.round(blackflyScore * 100) / 100, band: bandOf(blackflyScore) },
    basis: {
      mire_pct: pct(mireFrac),
      mire_source: usingMml ? "MML maastotietokanta (suo)" : "SYKE Corine -maanpeite",
      lake_pct: pct(lake),
      watercourse_km: Math.round((water ?? 0) * 100) / 100,
      radius_km: WATERCOURSE_RADIUS_M / 1000,
      latitude: Math.round(lat * 1000) / 1000,
    },
    source: usingMml
      ? "MML maastotietokanta (suo) + SYKE Corine + uomaverkosto (avoin paikkatieto)"
      : "SYKE Corine Land Cover 2018 + uomaverkosto (avoin paikkatieto)",
    partial: water == null || (clc == null && mmlMire == null),
  };
}
