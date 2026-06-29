/**
 * Chunked, resumable crawl driven by `crawl_state` (one row per source). Each
 * tick: pick the running-or-next-idle source, fetch ONE page, normalize, upsert
 * listings + diff events, lazily fetch new photos into R2, advance the cursor,
 * and exit — relying on the next cron tick to resume. Plane-A failures never
 * break the loop. Cross-portal dedup runs via property fingerprint in upsert.
 */
import {
  advanceCrawlState,
  ensureCrawlSource,
  getCrawlState,
  isPhotoSeen,
  listCrawlState,
  pickNextSource,
  recordDiffEvents,
  recordPhoto,
  upsertListing,
  type CrawlStateRow,
} from "./db";
import type { NormalizedListing } from "./normalize";
import { fetchOikotiePage } from "./sources/oikotie";
import { fetchEtuoviPage } from "./sources/etuovi";
import { fetchBooliPage } from "./sources/booli";
import { fetchFinnPage } from "./sources/finn";
import { fetchBoligsidenPage } from "./sources/boligsiden";
import { fetchVisirPage } from "./sources/visir";

/** Portals kontu can crawl, and the country each belongs to. */
const PORTAL_COUNTRY = {
  oikotie: "FI",
  etuovi: "FI",
  booli: "SE",
  finn: "NO",
  boligsiden: "DK",
  visir: "IS",
} as const;

const DEFAULT_SOURCES = [
  "oikotie:omakotitalo:outokumpu",
  "oikotie:omakotitalo:joensuu",
  "etuovi:omakotitalo:outokumpu",
  "etuovi:omakotitalo:liperi",
] as const;

const MAX_PHOTOS_PER_TICK = 24;
const PAGE_SIZE = 24;

export interface CrawlTickResult {
  source: string | null;
  fetched: number;
  inserted: number;
  updated: number;
  photos: number;
  ok: boolean;
  error?: string;
  state: CrawlStateRow[];
}

export async function ensureDefaultSources(db: D1Database): Promise<void> {
  for (const s of DEFAULT_SOURCES) {
    await ensureCrawlSource(db, s);
  }
}

/** Run exactly one bounded crawl tick. Always resolves (never throws). */
export async function crawlTick(db: D1Database, photos: R2Bucket): Promise<CrawlTickResult> {
  await ensureDefaultSources(db);
  const picked = await pickNextSource(db);
  if (!picked) {
    return { source: null, fetched: 0, inserted: 0, updated: 0, photos: 0, ok: true, state: await listCrawlState(db) };
  }

  const source = picked.source;
  const parsed = parseSource(source);
  if (!parsed) {
    await advanceCrawlState(db, source, { status: "error", last_error: "unparseable source key" });
    return { source, fetched: 0, inserted: 0, updated: 0, photos: 0, ok: false, error: "unparseable source", state: await listCrawlState(db) };
  }

  await advanceCrawlState(db, source, { status: "running" });
  const state = (await getCrawlState(db, source)) ?? picked;

  try {
    const { listings, found, ok, error } = await fetchPage(db, parsed, state);
    if (!ok) {
      await advanceCrawlState(db, source, { status: "error", last_error: error ?? "fetch failed" });
      return { source, fetched: 0, inserted: 0, updated: 0, photos: 0, ok: false, error, state: await listCrawlState(db) };
    }

    let inserted = 0;
    let updated = 0;
    let photoCount = 0;

    for (const listing of listings) {
      const res = await upsertListing(db, listing);
      await recordDiffEvents(db, res);
      if (res.inserted) inserted++;
      else if (res.changed) updated++;

      if (photoCount < MAX_PHOTOS_PER_TICK) {
        photoCount += await ingestPhotos(db, photos, res.listingId, listing, MAX_PHOTOS_PER_TICK - photoCount);
      }
    }

    const totalPages = Math.max(1, Math.ceil(found / PAGE_SIZE));
    const nextPage = state.next_page + 1;
    const done = nextPage > totalPages || listings.length === 0;
    await advanceCrawlState(db, source, {
      next_page: done ? 1 : nextPage,
      total_pages: totalPages,
      status: done ? "done" : "running",
      last_error: null,
    });

    return {
      source,
      fetched: listings.length,
      inserted,
      updated,
      photos: photoCount,
      ok: true,
      state: await listCrawlState(db),
    };
  } catch (err) {
    await advanceCrawlState(db, source, { status: "error", last_error: String(err) });
    return { source, fetched: 0, inserted: 0, updated: 0, photos: 0, ok: false, error: String(err), state: await listCrawlState(db) };
  }
}

interface ParsedSource {
  portal: keyof typeof PORTAL_COUNTRY;
  propertyType: string;
  location: string;
}

function parseSource(source: string): ParsedSource | null {
  const parts = source.split(":");
  if (parts.length < 3) return null;
  const [portal, propertyType, location] = parts;
  if (portal == null || !(portal in PORTAL_COUNTRY)) return null;
  if (!propertyType || !location) return null;
  return { portal: portal as keyof typeof PORTAL_COUNTRY, propertyType, location };
}

interface PageResult {
  listings: NormalizedListing[];
  found: number;
  ok: boolean;
  error?: string;
}

async function fetchPage(db: D1Database, parsed: ParsedSource, state: CrawlStateRow): Promise<PageResult> {
  const page = state.next_page;
  switch (parsed.portal) {
    case "oikotie": {
      const r = await fetchOikotiePage(db, {
        locations: parsed.location,
        offset: (page - 1) * PAGE_SIZE,
        limit: PAGE_SIZE,
      });
      return { listings: r.cards, found: r.found, ok: r.ok, error: r.error };
    }
    case "etuovi": {
      const r = await fetchEtuoviPage(db, {
        locations: [parsed.location],
        propertyTypes: [parsed.propertyType],
        page,
        size: PAGE_SIZE,
      });
      return { listings: r.announcements, found: r.total, ok: r.ok, error: r.error };
    }
    case "booli": {
      const r = await fetchBooliPage(db, { location: parsed.location, propertyTypes: [parsed.propertyType], page });
      return { listings: r.listings, found: r.found, ok: r.ok, error: r.error };
    }
    case "finn": {
      const r = await fetchFinnPage(db, { municipality: parsed.location, propertyTypes: [parsed.propertyType], page });
      return { listings: r.listings, found: r.found, ok: r.ok, error: r.error };
    }
    case "boligsiden": {
      const r = await fetchBoligsidenPage(db, { municipalities: [parsed.location], addressTypes: [parsed.propertyType], page });
      return { listings: r.listings, found: r.found, ok: r.ok, error: r.error };
    }
    case "visir": {
      const r = await fetchVisirPage(db, { zip: parsed.location, page });
      return { listings: r.listings, found: r.found, ok: r.ok, error: r.error };
    }
  }
}

function extractPhotoUrls(raw: string): string[] {
  const urls = new Set<string>();
  try {
    const obj = JSON.parse(raw) as Record<string, unknown>;
    collectImageUrls(obj, urls);
  } catch {
    // raw_json may be malformed; tolerate silently
  }
  return [...urls].slice(0, MAX_PHOTOS_PER_TICK);
}

function collectImageUrls(node: unknown, out: Set<string>): void {
  if (node == null) return;
  if (typeof node === "string") {
    if (/^https?:\/\/.+\.(jpe?g|png|webp)(\?|$)/i.test(node)) out.add(node);
    return;
  }
  if (Array.isArray(node)) {
    for (const v of node) collectImageUrls(v, out);
    return;
  }
  if (typeof node === "object") {
    for (const v of Object.values(node as Record<string, unknown>)) collectImageUrls(v, out);
  }
}

async function ingestPhotos(
  db: D1Database,
  photos: R2Bucket,
  listingId: number,
  listing: NormalizedListing,
  budget: number,
): Promise<number> {
  const urls = extractPhotoUrls(listing.raw_json);
  let count = 0;
  let position = 0;
  for (const url of urls) {
    if (count >= budget) break;
    position++;
    try {
      const seen = await isPhotoSeen(db, url);
      if (seen) {
        await recordPhoto(db, listingId, position, seen, url, null);
        continue;
      }
      const res = await fetch(url);
      if (!res.ok) continue;
      const bytes = await res.arrayBuffer();
      const key = `photos/${await sha256Hex(bytes)}`;
      const contentType = res.headers.get("content-type");
      await photos.put(key, bytes, contentType ? { httpMetadata: { contentType } } : undefined);
      await recordPhoto(db, listingId, position, key, url, contentType);
      count++;
    } catch (err) {
      console.warn("photo ingest failed", url, String(err));
    }
  }
  return count;
}

async function sha256Hex(bytes: ArrayBuffer): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, "0")).join("");
}
