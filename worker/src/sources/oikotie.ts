/**
 * Plane-A Oikotie adapter. Disposable by design: every call is wrapped so a
 * failure (401 without OTA tokens, bot-detection, datacenter-IP block) logs and
 * returns empty rather than breaking the crawl. All volatile params/headers come
 * from `source_config` in D1, never hardcoded.
 */
import { parse } from "node-html-parser";
import { getSourceConfig } from "../db";
import { normalizeOikotieCard, type NormalizedListing } from "../normalize";

interface OtaTokens {
  "OTA-token"?: string;
  "OTA-cuid"?: string;
  "OTA-loaded"?: string;
}

export interface OikotieFetchResult {
  cards: NormalizedListing[];
  found: number;
  ok: boolean;
  error?: string;
}

const DEFAULT_UA =
  "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

/**
 * Harvest short-lived OTA-* request headers from the search page `<head>` meta
 * tags. Tag names (`api-token`/`cuid`/`loaded`) come from source_config
 * `oikotie.ota_meta_map`. Returns `{}` on any failure.
 */
export async function harvestOtaTokens(
  cfg: Record<string, string>,
): Promise<OtaTokens> {
  const pageUrl = cfg["search_page_url"] ?? "https://asunnot.oikotie.fi/myytavat-asunnot";
  const ua = cfg["user_agent"] ?? DEFAULT_UA;
  const metaMap = parseJsonObject(cfg["ota_meta_map"]) ?? {
    "api-token": "OTA-token",
    cuid: "OTA-cuid",
    loaded: "OTA-loaded",
  };
  try {
    const res = await fetch(pageUrl, {
      headers: { "User-Agent": ua, Accept: "text/html", Referer: "https://asunnot.oikotie.fi/" },
    });
    if (!res.ok) return {};
    const html = await res.text();
    const root = parse(html);
    const tokens: OtaTokens = {};
    for (const [metaName, headerName] of Object.entries(metaMap)) {
      if (typeof headerName !== "string") continue;
      const el = root.querySelector(`meta[name="${metaName}"]`);
      const content = el?.getAttribute("content");
      if (content) (tokens as Record<string, string>)[headerName] = content;
    }
    return tokens;
  } catch (err) {
    console.warn("oikotie token harvest failed", String(err));
    return {};
  }
}

export interface OikotieQuery {
  locations: string;
  priceMin?: number;
  priceMax?: number;
  offset: number;
  limit?: number;
}

function buildCardsUrl(cfg: Record<string, string>, q: OikotieQuery): string {
  const base = cfg["cards_url"] ?? "https://asunnot.oikotie.fi/api/cards";
  const params = new URLSearchParams();
  params.set("cardType", cfg["card_type_for_sale"] ?? "100");
  params.set("locations", q.locations);
  if (q.priceMin != null) params.set("price[min]", String(q.priceMin));
  if (q.priceMax != null) params.set("price[max]", String(q.priceMax));
  params.set("limit", String(q.limit ?? 24));
  params.set("offset", String(q.offset));
  params.set("sortBy", "published_desc");

  const buildingCodes = parseJsonObject(cfg["building_type_codes"]) ?? {};
  for (const code of Object.values(buildingCodes)) {
    params.append("buildingType[]", String(code));
  }
  return `${base}?${params.toString()}`;
}

/**
 * Fetch one page of Oikotie cards. Best-effort: returns `{ ok:false, cards:[] }`
 * on any failure (missing tokens → 401, block, parse error). Never throws.
 */
export async function fetchOikotiePage(
  db: D1Database,
  q: OikotieQuery,
): Promise<OikotieFetchResult> {
  const cfg = await getSourceConfig(db, "oikotie");
  const ua = cfg["user_agent"] ?? DEFAULT_UA;
  try {
    const tokens = await harvestOtaTokens(cfg);
    if (!tokens["OTA-token"] || !tokens["OTA-cuid"] || !tokens["OTA-loaded"]) {
      return { cards: [], found: 0, ok: false, error: "missing OTA tokens (page block or schema drift)" };
    }
    const url = buildCardsUrl(cfg, q);
    const res = await fetch(url, {
      headers: {
        ...(tokens as Record<string, string>),
        "User-Agent": ua,
        Accept: "application/json",
        Referer: cfg["search_page_url"] ?? "https://asunnot.oikotie.fi/myytavat-asunnot",
      },
    });
    if (!res.ok) {
      return { cards: [], found: 0, ok: false, error: `oikotie cards HTTP ${res.status}` };
    }
    const body = (await res.json()) as { cards?: unknown[]; found?: number };
    const cards = Array.isArray(body.cards) ? body.cards : [];
    return {
      cards: cards.map(normalizeOikotieCard),
      found: typeof body.found === "number" ? body.found : cards.length,
      ok: true,
    };
  } catch (err) {
    console.warn("oikotie fetch failed", String(err));
    return { cards: [], found: 0, ok: false, error: String(err) };
  }
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
