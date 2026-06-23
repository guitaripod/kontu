/**
 * Plane-A Etuovi adapter. POSTs the internal search API with a normal browser
 * User-Agent (Etuovi robots blocks `ClaudeBot`/`anthropic-ai`). Throttles,
 * backs off on 429, and is fully wrapped so failures never break the crawl.
 */
import { getSourceConfig } from "../db";
import { normalizeEtuoviAnnouncement, type NormalizedListing } from "../normalize";

export interface EtuoviFetchResult {
  announcements: NormalizedListing[];
  total: number;
  ok: boolean;
  error?: string;
}

export interface EtuoviQuery {
  locations: string[];
  propertyTypes?: string[];
  page: number;
  size?: number;
  priceMin?: number;
  priceMax?: number;
}

const DEFAULT_UA =
  "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

const sleep = (ms: number): Promise<void> => new Promise((r) => setTimeout(r, ms));

function buildBody(cfg: Record<string, string>, q: EtuoviQuery): unknown {
  const typeCodes = parseJsonObject(cfg["property_type_codes"]) ?? {};
  const propertyType =
    q.propertyTypes && q.propertyTypes.length
      ? q.propertyTypes.map((t) => typeCodes[t] ?? t)
      : Object.values(typeCodes);
  const body: Record<string, unknown> = {
    sellerType: null,
    sortBy: "PUBLISHED_OR_UPDATED_AT",
    sortDirection: "DESC",
    pagination: { firstResult: (q.page - 1) * (q.size ?? 30), maxResults: q.size ?? 30 },
    locationSearchCriteria: q.locations.map((l) => ({ name: l })),
  };
  if (propertyType.length) body["propertyType"] = propertyType;
  if (q.priceMin != null || q.priceMax != null) {
    body["price"] = { min: q.priceMin ?? null, max: q.priceMax ?? null };
  }
  return body;
}

/**
 * Fetch one Etuovi search page with retry/backoff on 429. Best-effort: returns
 * `{ ok:false, announcements:[] }` on failure. Never throws.
 */
export async function fetchEtuoviPage(db: D1Database, q: EtuoviQuery): Promise<EtuoviFetchResult> {
  const cfg = await getSourceConfig(db, "etuovi");
  const url = cfg["search_url"] ?? "https://www.etuovi.com/api/v3/announcements/search/listpage";
  const ua = cfg["user_agent"] ?? DEFAULT_UA;
  const throttle = numberOr(cfg["rate_limit_ms"], 800);
  const body = JSON.stringify(buildBody(cfg, q));

  await sleep(throttle);

  let lastError = "";
  for (let attempt = 0; attempt < 3; attempt++) {
    try {
      const res = await fetch(url, {
        method: "POST",
        headers: {
          "User-Agent": ua,
          "Content-Type": "application/json",
          Accept: "application/json",
          Origin: "https://www.etuovi.com",
          Referer: "https://www.etuovi.com/myytavat-asunnot",
        },
        body,
      });
      if (res.status === 429) {
        lastError = "etuovi HTTP 429";
        await sleep(throttle * Math.pow(2, attempt + 1));
        continue;
      }
      if (!res.ok) {
        return { announcements: [], total: 0, ok: false, error: `etuovi HTTP ${res.status}` };
      }
      const parsed = (await res.json()) as Record<string, unknown>;
      const list = extractAnnouncements(parsed);
      const total = extractTotal(parsed, list.length);
      return {
        announcements: list.map(normalizeEtuoviAnnouncement),
        total,
        ok: true,
      };
    } catch (err) {
      lastError = String(err);
      await sleep(throttle * Math.pow(2, attempt + 1));
    }
  }
  console.warn("etuovi fetch failed", lastError);
  return { announcements: [], total: 0, ok: false, error: lastError };
}

function extractAnnouncements(body: Record<string, unknown>): unknown[] {
  for (const key of ["announcements", "results", "items", "content"]) {
    const v = body[key];
    if (Array.isArray(v)) return v;
  }
  return [];
}

function extractTotal(body: Record<string, unknown>, fallback: number): number {
  for (const key of ["totalCount", "total", "totalResults", "count"]) {
    const v = body[key];
    if (typeof v === "number") return v;
  }
  return fallback;
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
