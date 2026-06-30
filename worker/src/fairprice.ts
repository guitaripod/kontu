/**
 * Plane-B price fairness (SPEC §8). Joins StatFin's kunta name→code map with
 * MML's detached-house median sold price per kunta to benchmark a listing's
 * asking price. Open-gov, zero legal risk, cached in `market_stats`.
 *
 * Attribution: Maanmittauslaitos, Kauppahintarekisteri (CC BY 4.0).
 */
import { asciiFold } from "./normalize";

const STATFIN_META_URL = "https://pxdata.stat.fi/PXWeb/api/v1/fi/StatFin/ashi/13mx.px";
const MML_INDICATOR_URL =
  "https://khr.maanmittauslaitos.fi/tilastopalvelu/rest/1.1/categories/Kunta/indicators/2313/data";
const FETCH_TIMEOUT_MS = 8000;
const MARKET_STALE_SECONDS = 24 * 60 * 60;
const MML_MIN_ROWS = 10;
const UPSERT_BATCH = 50;

interface StatFinMeta {
  variables?: Array<{ code?: string; values?: unknown[]; valueTexts?: unknown[] }>;
}

interface MmlRow {
  region?: unknown;
  value?: unknown;
}

/** Lowercased+folded kunta name → integer kunta code, from the StatFin 13mx metadata. */
async function fetchKuntaCodeMap(): Promise<Map<string, number>> {
  const out = new Map<string, number>();
  const res = await fetch(STATFIN_META_URL, {
    headers: { Accept: "application/json" },
    signal: AbortSignal.timeout(FETCH_TIMEOUT_MS),
  });
  if (!res.ok) return out;
  const meta = (await res.json()) as StatFinMeta;
  const kunta = (meta.variables ?? []).find((v) => typeof v.code === "string" && v.code.startsWith("kunta"));
  if (!kunta || !Array.isArray(kunta.values) || !Array.isArray(kunta.valueTexts)) return out;
  const n = Math.min(kunta.values.length, kunta.valueTexts.length);
  for (let i = 0; i < n; i++) {
    const code = parseInt(String(kunta.values[i]), 10);
    const name = asciiFold(kunta.valueTexts[i]);
    if (Number.isFinite(code) && name !== "") out.set(name, code);
  }
  return out;
}

/** Integer kunta code → median total detached-house sale price, for one year. */
async function fetchMmlMedians(year: number): Promise<Map<number, number>> {
  const out = new Map<number, number>();
  const res = await fetch(`${MML_INDICATOR_URL}?years=${year}`, {
    headers: { Accept: "application/json" },
    signal: AbortSignal.timeout(FETCH_TIMEOUT_MS),
  });
  if (!res.ok) return out;
  const rows = (await res.json()) as unknown;
  if (!Array.isArray(rows)) return out;
  for (const r of rows as MmlRow[]) {
    const code = Math.trunc(Number(r?.region));
    const value = Number(r?.value);
    if (Number.isFinite(code) && Number.isFinite(value) && value > 0) out.set(code, value);
  }
  return out;
}

/**
 * Refresh `market_stats` with MML detached-house median prices keyed by kunta
 * NAME (lowercased+folded) so the API can join on a listing's municipality.
 * Best-effort: never throws; returns the number of municipalities written.
 */
export async function refreshMarketStats(db: D1Database): Promise<number> {
  try {
    const codeByName = await fetchKuntaCodeMap();
    if (codeByName.size === 0) return 0;

    const currentYear = new Date().getUTCFullYear();
    let year = currentYear - 1;
    let medians = await fetchMmlMedians(year);
    if (medians.size < MML_MIN_ROWS) {
      year = currentYear - 2;
      medians = await fetchMmlMedians(year);
    }
    if (medians.size === 0) return 0;

    const period = String(year);
    const statements: D1PreparedStatement[] = [];
    for (const [name, code] of codeByName) {
      const median = medians.get(code);
      if (median == null) continue;
      statements.push(
        db
          .prepare(
            "INSERT OR REPLACE INTO market_stats " +
              "(country, area_kind, area_code, metric, property_kind, period, value, source, fetched_at) " +
              "VALUES ('FI', 'municipality', ?, 'median_total_eur', 'okt_kiinteisto', ?, ?, 'mml', unixepoch())",
          )
          .bind(name, period, median),
      );
    }
    if (statements.length === 0) return 0;

    for (let i = 0; i < statements.length; i += UPSERT_BATCH) {
      await db.batch(statements.slice(i, i + UPSERT_BATCH));
    }
    return statements.length;
  } catch {
    return 0;
  }
}

/**
 * Self-benchmark the non-FI markets from kontu's OWN active listings: the median total
 * asking price per (country, municipality) where the sample is large enough to mean
 * something. This is asking-price (not sold-price like Finland's MML), so a weaker,
 * relative signal — but it gives the cross-Nordic value lane an area benchmark instead
 * of "cheap vs your own budget". Written with the same metric the consumer already
 * keys on (median_total_eur), tagged source='kontu_asking'. Never throws.
 */
export async function refreshNordicMarketStats(db: D1Database): Promise<number> {
  const MIN_SAMPLE = 5;
  try {
    const { results } = await db
      .prepare(
        "SELECT country, municipality, price_eur, living_area_m2 FROM listings " +
          "WHERE country != 'FI' AND status = 'active' AND price_eur > 1000 " +
          "AND living_area_m2 > 15 AND municipality IS NOT NULL",
      )
      .all<{ country: string; municipality: string; price_eur: number; living_area_m2: number }>();
    // Median price PER M² — size-normalized, so a small cheap house isn't mislabelled
    // "underpriced" against a municipality median dominated by large houses.
    const buckets = new Map<string, { country: string; name: string; ppm2: number[] }>();
    for (const r of results) {
      const name = asciiFold(r.municipality);
      if (name === "" || r.living_area_m2 <= 0) continue;
      const key = `${r.country}|${name}`;
      const b = buckets.get(key) ?? { country: r.country, name, ppm2: [] };
      b.ppm2.push(r.price_eur / r.living_area_m2);
      buckets.set(key, b);
    }
    const period = String(new Date().getUTCFullYear());
    const statements: D1PreparedStatement[] = [];
    for (const b of buckets.values()) {
      if (b.ppm2.length < MIN_SAMPLE) continue;
      b.ppm2.sort((a, c) => a - c);
      const mid = Math.floor(b.ppm2.length / 2);
      const median = b.ppm2.length % 2 === 0 ? (b.ppm2[mid - 1]! + b.ppm2[mid]!) / 2 : b.ppm2[mid]!;
      statements.push(
        db
          .prepare(
            "INSERT OR REPLACE INTO market_stats " +
              "(country, area_kind, area_code, metric, property_kind, period, value, source, fetched_at) " +
              "VALUES (?, 'municipality', ?, 'median_ppm2_eur', 'okt_kiinteisto', ?, ?, 'kontu_asking', unixepoch())",
          )
          .bind(b.country, b.name, period, median),
      );
    }
    if (statements.length === 0) return 0;
    for (let i = 0; i < statements.length; i += UPSERT_BATCH) {
      await db.batch(statements.slice(i, i + UPSERT_BATCH));
    }
    return statements.length;
  } catch {
    return 0;
  }
}

const FAIRNESS_BANDS: Array<[number, string]> = [
  [0.8, "underpriced"],
  [0.92, "below_market"],
  [1.08, "fair"],
  [1.2, "above_market"],
];

/** Map an asking-price/benchmark ratio to a fairness band. null/non-finite → 'unknown'. */
export function fairnessBand(ratio: number | null): string {
  if (ratio == null || !Number.isFinite(ratio)) return "unknown";
  for (const [upper, band] of FAIRNESS_BANDS) {
    if (ratio < upper) return band;
  }
  return "overpriced";
}

/** `country|name` (lowercased+folded municipality) → median total price, from `market_stats`. */
export async function loadMedians(db: D1Database): Promise<Map<string, number>> {
  const out = new Map<string, number>();
  const { results } = await db
    .prepare(
      "SELECT country, area_code, value FROM market_stats m WHERE metric = 'median_total_eur' " +
        "AND period = (SELECT MAX(period) FROM market_stats WHERE metric = 'median_total_eur' " +
        "AND country = m.country AND area_code = m.area_code)",
    )
    .all<{ country: string; area_code: string; value: number }>();
  for (const r of results) {
    if (r.value != null) out.set(`${r.country}|${r.area_code}`, r.value);
  }
  return out;
}

/** `country|name` → median price-per-m² (the size-normalized non-FI benchmark). */
export async function loadPpm2Medians(db: D1Database): Promise<Map<string, number>> {
  const out = new Map<string, number>();
  const { results } = await db
    .prepare(
      "SELECT country, area_code, value FROM market_stats m WHERE metric = 'median_ppm2_eur' " +
        "AND period = (SELECT MAX(period) FROM market_stats WHERE metric = 'median_ppm2_eur' " +
        "AND country = m.country AND area_code = m.area_code)",
    )
    .all<{ country: string; area_code: string; value: number }>();
  for (const r of results) {
    if (r.value != null) out.set(`${r.country}|${r.area_code}`, r.value);
  }
  return out;
}

/** True if no median row exists or the freshest one is older than 24h. */
export async function marketIsStale(db: D1Database): Promise<boolean> {
  const row = await db
    .prepare("SELECT MAX(fetched_at) AS latest FROM market_stats WHERE metric = 'median_total_eur'")
    .first<{ latest: number | null }>();
  const latest = row?.latest ?? null;
  if (latest == null) return true;
  return Math.floor(Date.now() / 1000) - latest > MARKET_STALE_SECONDS;
}

export interface Fairness {
  band: string;
  ratio: number | null;
  benchmark: number | null;
  confidence: "medium" | "unknown";
}

/** Compute a listing's fairness. Finland uses the MML median TOTAL sold price; the other
 *  Nordic markets use the size-normalized median price-per-m² self-benchmark (compared
 *  against the listing's own price/m²), which avoids mislabelling small cheap houses. */
export function computeFairness(
  medians: Map<string, number>,
  country: string | null,
  municipality: string | null,
  priceEur: number | null,
  ppm2Medians?: Map<string, number>,
  livingAreaM2?: number | null,
): Fairness {
  const name = asciiFold(municipality);
  const key = `${(country ?? "FI").toUpperCase()}|${name}`;
  // Prefer the per-m² benchmark when one exists for this area and the listing has a size.
  const ppm2Benchmark = name !== "" ? ppm2Medians?.get(key) : undefined;
  if (ppm2Benchmark != null && priceEur && livingAreaM2 && livingAreaM2 > 0) {
    const ratio = priceEur / livingAreaM2 / ppm2Benchmark;
    return { band: fairnessBand(ratio), ratio, benchmark: ppm2Benchmark, confidence: "medium" };
  }
  const benchmark = (name !== "" ? medians.get(key) : undefined) ?? null;
  const ratio = benchmark && priceEur ? priceEur / benchmark : null;
  return {
    band: fairnessBand(ratio),
    ratio,
    benchmark,
    confidence: benchmark ? "medium" : "unknown",
  };
}
