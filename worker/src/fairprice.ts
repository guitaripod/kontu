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
              "(area_kind, area_code, metric, property_kind, period, value, source, fetched_at) " +
              "VALUES ('municipality', ?, 'median_total_eur', 'okt_kiinteisto', ?, ?, 'mml', unixepoch())",
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

/** kunta name (lowercased+folded) → median total price, from cached `market_stats`. */
export async function loadMedians(db: D1Database): Promise<Map<string, number>> {
  const out = new Map<string, number>();
  const { results } = await db
    .prepare(
      "SELECT area_code, value FROM market_stats m WHERE metric = 'median_total_eur' " +
        "AND period = (SELECT MAX(period) FROM market_stats WHERE metric = 'median_total_eur' AND area_code = m.area_code)",
    )
    .all<{ area_code: string; value: number }>();
  for (const r of results) {
    if (r.value != null) out.set(r.area_code, r.value);
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

/** Compute a listing's fairness object from its municipality + asking price. */
export function computeFairness(
  medians: Map<string, number>,
  municipality: string | null,
  priceEur: number | null,
): Fairness {
  const name = asciiFold(municipality);
  const benchmark = (name !== "" ? medians.get(name) : undefined) ?? null;
  const ratio = benchmark && priceEur ? priceEur / benchmark : null;
  return {
    band: fairnessBand(ratio),
    ratio,
    benchmark,
    confidence: benchmark ? "medium" : "unknown",
  };
}
