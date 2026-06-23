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

const OVERPASS_URL = "https://overpass-api.de/api/interpreter";
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
    const res = await fetch(OVERPASS_URL, {
      method: "POST",
      headers: { "Content-Type": "text/plain", Accept: "application/json" },
      body: query,
    });
    if (!res.ok) return out;
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
    const res = await fetch(OVERPASS_URL, {
      method: "POST",
      headers: { "Content-Type": "text/plain", Accept: "application/json" },
      body: query,
    });
    if (!res.ok) return null;
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
