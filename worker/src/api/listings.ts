/**
 * Token-guarded REST API (SPEC §9). Mounted under `/api` in index.ts. All D1
 * access goes through the typed helpers in db.ts; Plane-B reads (listings,
 * cost-defaults) work fully offline. The cost-defaults shape is the contract
 * with the Rust `CostDefaults` struct.
 */
import { Hono } from "hono";
import type { Env } from "../index";
import {
  createSavedSearch,
  deleteSavedSearch,
  getCostDefaults,
  getCostInputs,
  getDossier,
  getListing,
  getListingEvents,
  getListingPhotos,
  getListingsForProperty,
  getMarketStats,
  getNote,
  getProperty,
  getScore,
  getTags,
  listSavedSearches,
  queryListings,
  setCostInputs,
  setNote,
  setScore,
  setTags,
  updateSavedSearch,
  upsertListing,
  recordDiffEvents,
  recordPhoto,
  isPhotoSeen,
  deletePhotosFrom,
  upsertPublishedPage,
  deletePublishedPage,
  type ListingsFilter,
} from "../db";
import { crawlTick } from "../crawl";
import { enrichBatch } from "../geo";
import {
  normalizeOikotieCard,
  normalizeEtuoviAnnouncement,
  oikotiePhotoUrls,
  etuoviPhotoUrls,
  oikotieCountry,
  isForeignListing,
} from "../normalize";
import { computeFairness, loadMedians, marketIsStale, refreshMarketStats } from "../fairprice";

export const api = new Hono<{ Bindings: Env }>();

const INTEGER_DEFAULT_KEYS = new Set(["loan_term_years"]);

api.get("/listings", async (c) => {
  const filter = parseListingsFilter(c.req.query.bind(c.req), c.req.queries.bind(c.req));
  const { listings, total } = await queryListings(c.env.DB, filter);
  const medians = await loadMedians(c.env.DB);
  const enriched = listings.map((l) => ({
    ...l,
    risk_structures: parseJsonArray(l.risk_structures),
    days_on_market: daysOnMarket(l.first_seen),
    price_per_m2: l.price_per_m2 ?? derivePpm2(l.price_eur, l.living_area_m2),
    fairness: computeFairness(medians, l.municipality, l.price_eur),
  }));
  return c.json({ listings: enriched, total });
});

api.get("/listings/:id", async (c) => {
  const id = Number(c.req.param("id"));
  if (!Number.isInteger(id)) return c.json({ error: "bad id" }, 400);
  const listing = await getListing(c.env.DB, id);
  if (!listing) return c.json({ error: "not found" }, 404);

  const [events, photos, costInputs, note, score, tags] = await Promise.all([
    getListingEvents(c.env.DB, id),
    getListingPhotos(c.env.DB, id),
    getCostInputs(c.env.DB, id),
    getNote(c.env.DB, id),
    getScore(c.env.DB, id),
    getTags(c.env.DB, id),
  ]);
  const dossier = listing.property_id != null ? await getDossier(c.env.DB, listing.property_id) : null;
  const medians = await loadMedians(c.env.DB);

  return c.json({
    listing: {
      ...listing,
      risk_structures: parseJsonArray(listing.risk_structures),
      days_on_market: daysOnMarket(listing.first_seen),
      price_per_m2: listing.price_per_m2 ?? derivePpm2(listing.price_eur, listing.living_area_m2),
      fairness: computeFairness(medians, listing.municipality, listing.price_eur),
    },
    events,
    photos,
    dossier,
    cost_inputs: costInputs,
    note,
    score,
    tags,
  });
});

/**
 * Publish (or refresh) a public listing page from a CLI-computed snapshot. The
 * payload is stored verbatim and rendered at `/h/:id`; the Worker never
 * recomputes cost/risk. Auto-called per gate-passer + pin during `watch run`.
 */
api.post("/publish", async (c) => {
  const body = await c.req.json<{ id?: number; tier?: string; payload?: unknown }>().catch(() => null);
  const id = Number(body?.id);
  if (!body || !Number.isInteger(id) || body.payload == null) {
    return c.json({ error: "bad request: need { id, payload }" }, 400);
  }
  // The public site is the algorithm-VALIDATED showcase: only gate-passers are
  // published. Near-misses and manual pins stay in the CLI, never on the website.
  if (body.tier !== "gate") {
    return c.json({ ok: false, skipped: "only algorithm-validated (gate) listings are published" }, 422);
  }
  await upsertPublishedPage(c.env.DB, id, body.tier, JSON.stringify(body.payload));
  return c.json({ ok: true, id, path: `/kontu/${id}` });
});

api.delete("/publish/:id", async (c) => {
  const id = Number(c.req.param("id"));
  if (!Number.isInteger(id)) return c.json({ error: "bad id" }, 400);
  await deletePublishedPage(c.env.DB, id);
  return c.json({ ok: true });
});

api.get("/properties/:id", async (c) => {
  const id = Number(c.req.param("id"));
  if (!Number.isInteger(id)) return c.json({ error: "bad id" }, 400);
  const property = await getProperty(c.env.DB, id);
  if (!property) return c.json({ error: "not found" }, 404);
  const listings = await getListingsForProperty(c.env.DB, id);
  const dossier = await getDossier(c.env.DB, id);
  return c.json({
    property,
    dossier,
    listings: listings.map((l) => ({
      ...l,
      risk_structures: parseJsonArray(l.risk_structures),
      days_on_market: daysOnMarket(l.first_seen),
      price_per_m2: l.price_per_m2 ?? derivePpm2(l.price_eur, l.living_area_m2),
    })),
  });
});

api.post("/sync", async (c) => {
  const tick = await crawlTick(c.env.DB, c.env.PHOTOS);
  const enriched = await enrichBatch(c.env.DB, 3);
  try {
    if (await marketIsStale(c.env.DB)) await refreshMarketStats(c.env.DB);
  } catch {
    /* best-effort: never block a sync on Plane-B benchmark refresh */
  }
  return c.json({ tick, enriched });
});

api.post("/import", async (c) => {
  const body: { source?: string; cards?: unknown[]; announcements?: unknown[] } = await c.req
    .json<{ source?: string; cards?: unknown[]; announcements?: unknown[] }>()
    .catch(() => ({}));
  const isEtuovi =
    body.source === "etuovi" ||
    (body.source !== "oikotie" && !Array.isArray(body.cards) && Array.isArray(body.announcements));
  const items = isEtuovi
    ? Array.isArray(body.announcements)
      ? body.announcements
      : []
    : Array.isArray(body.cards)
      ? body.cards
      : [];
  const normalize = isEtuovi ? normalizeEtuoviAnnouncement : normalizeOikotieCard;

  let inserted = 0;
  let updated = 0;
  let skipped = 0;
  for (const item of items) {
    try {
      const n = normalize(item);
      if (!n.portal_listing_id) {
        skipped++;
        continue;
      }
      if (isForeignListing(n.municipality, isEtuovi ? null : oikotieCountry(item))) {
        skipped++;
        continue;
      }
      const res = await upsertListing(c.env.DB, n);
      await recordDiffEvents(c.env.DB, res);
      if (res.inserted) inserted++;
      else updated++;
      await recordCoverPhoto(c.env.DB, res.listingId, isEtuovi ? etuoviPhotoUrls(item) : oikotiePhotoUrls(item));
    } catch {
      skipped++;
    }
  }

  try {
    if (await marketIsStale(c.env.DB)) await refreshMarketStats(c.env.DB);
  } catch {
    /* best-effort: warm Plane-B benchmarks after a pull */
  }

  return c.json({
    source: body.source ?? (isEtuovi ? "etuovi" : "oikotie"),
    received: items.length,
    inserted,
    updated,
    skipped,
  });
});

/**
 * Record a listing's cover photo(s) WITHOUT downloading them: store the source
 * URL under a URL-derived R2 key. `/api/photos/:key` fetches the bytes lazily on
 * first view (read-through cache), so imports stay fast and only viewed images
 * are ever pulled. Best-effort: a photo failure never breaks an import.
 */
async function recordCoverPhoto(db: D1Database, listingId: number, urls: string[]): Promise<void> {
  if (urls.length === 0) return;
  let position = 0;
  for (const url of urls) {
    position++;
    try {
      // Reuse a key the crawler already cached for this URL; else derive one from
      // the URL so the read-through cache can resolve it lazily on first view.
      const key = (await isPhotoSeen(db, url)) ?? `photos/${await sha256Hex(url)}`;
      await recordPhoto(db, listingId, position, key, url, null);
    } catch {
      /* best-effort: a photo failure must never break an import */
    }
  }
  try {
    await deletePhotosFrom(db, listingId, urls.length);
  } catch {
    /* best-effort */
  }
}

async function sha256Hex(input: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(input));
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

api.get("/cost-defaults", async (c) => {
  const defaults = await getCostDefaults(c.env.DB);
  const out: Record<string, number> = {};
  for (const [key, value] of Object.entries(defaults)) {
    out[key] = INTEGER_DEFAULT_KEYS.has(key) ? Math.round(value) : value;
  }
  return c.json(out);
});

api.get("/market/:municipality", async (c) => {
  const municipality = c.req.param("municipality");
  const stats = await getMarketStats(c.env.DB, municipality);
  return c.json({ municipality, stats });
});

api.get("/saved-searches", async (c) => {
  const searches = await listSavedSearches(c.env.DB);
  return c.json({
    saved_searches: searches.map((s) => ({ ...s, params: parseJsonObject(s.params_json) })),
  });
});

api.post("/saved-searches", async (c) => {
  const body = await readJson(c);
  if (!body || typeof body.name !== "string" || body.name.trim() === "") {
    return c.json({ error: "name required" }, 400);
  }
  const created = await createSavedSearch(
    c.env.DB,
    body.name.trim(),
    body.params ?? {},
    Boolean(body.is_exact),
  );
  return c.json({ ...created, params: parseJsonObject(created.params_json) }, 201);
});

api.put("/saved-searches/:id", async (c) => {
  const id = Number(c.req.param("id"));
  if (!Number.isInteger(id)) return c.json({ error: "bad id" }, 400);
  const body = await readJson(c);
  const updated = await updateSavedSearch(c.env.DB, id, {
    name: typeof body?.name === "string" ? body.name : undefined,
    params: body && "params" in body ? body.params : undefined,
    isExact: body && "is_exact" in body ? Boolean(body.is_exact) : undefined,
  });
  if (!updated) return c.json({ error: "not found" }, 404);
  return c.json({ ...updated, params: parseJsonObject(updated.params_json) });
});

api.delete("/saved-searches/:id", async (c) => {
  const id = Number(c.req.param("id"));
  if (!Number.isInteger(id)) return c.json({ error: "bad id" }, 400);
  const ok = await deleteSavedSearch(c.env.DB, id);
  if (!ok) return c.json({ error: "not found" }, 404);
  return c.json({ ok: true });
});

api.put("/listings/:id/notes", async (c) => {
  const id = requireListingId(c.req.param("id"));
  if (id == null) return c.json({ error: "bad id" }, 400);
  const body = await readJson(c);
  const note = typeof body?.note === "string" ? body.note : "";
  await setNote(c.env.DB, id, note);
  return c.json({ ok: true, note });
});

api.put("/listings/:id/score", async (c) => {
  const id = requireListingId(c.req.param("id"));
  if (id == null) return c.json({ error: "bad id" }, 400);
  const body = await readJson(c);
  const score = typeof body?.score === "number" ? clampScore(body.score) : null;
  const dealBreaker = Boolean(body?.deal_breaker);
  await setScore(c.env.DB, id, score, dealBreaker, body?.criteria);
  return c.json({ ok: true, score, deal_breaker: dealBreaker });
});

api.put("/listings/:id/cost-inputs", async (c) => {
  const id = requireListingId(c.req.param("id"));
  if (id == null) return c.json({ error: "bad id" }, 400);
  const body = await readJson(c);
  await setCostInputs(c.env.DB, id, body ?? {});
  return c.json({ ok: true, cost_inputs: body ?? {} });
});

api.put("/listings/:id/tags", async (c) => {
  const id = requireListingId(c.req.param("id"));
  if (id == null) return c.json({ error: "bad id" }, 400);
  const body = await readJson(c);
  const tags = Array.isArray(body?.tags) ? body.tags.filter((t: unknown): t is string => typeof t === "string") : [];
  await setTags(c.env.DB, id, tags);
  return c.json({ ok: true, tags });
});

function parseListingsFilter(
  q: (key: string) => string | undefined,
  queries: (key: string) => string[] | undefined,
): ListingsFilter {
  const num = (key: string): number | undefined => {
    const v = q(key);
    if (v == null || v === "") return undefined;
    const n = Number(v);
    return Number.isFinite(n) ? n : undefined;
  };
  const str = (key: string): string | undefined => {
    const v = q(key);
    return v == null || v === "" ? undefined : v;
  };
  const order = q("order") === "desc" ? "desc" : q("order") === "asc" ? "asc" : undefined;
  return {
    municipality: str("municipality"),
    property_type: str("property_type"),
    holding_form: str("holding_form"),
    price_min: num("price_min"),
    price_max: num("price_max"),
    m2_min: num("m2_min"),
    m2_max: num("m2_max"),
    rooms_min: num("rooms_min"),
    year_min: num("year_min"),
    shore: str("shore"),
    heating_type: str("heating_type"),
    energy_class_max: str("energy_class_max"),
    plot_ownership: str("plot_ownership"),
    max_days_on_market: num("max_days_on_market"),
    exclude: (queries("exclude") ?? []).filter((s) => s !== ""),
    price_dropped: q("price_dropped") === "1" || q("price_dropped") === "true",
    text: str("text"),
    sort: str("sort"),
    order,
    limit: num("limit"),
    offset: num("offset"),
  };
}

function requireListingId(param: string): number | null {
  const id = Number(param);
  return Number.isInteger(id) ? id : null;
}

function daysOnMarket(firstSeen: number): number {
  const seconds = Math.floor(Date.now() / 1000) - firstSeen;
  return Math.max(0, Math.floor(seconds / 86400));
}

function derivePpm2(price: number | null, m2: number | null): number | null {
  if (price == null || m2 == null || m2 <= 0) return null;
  return Math.round((price / m2) * 100) / 100;
}

function clampScore(n: number): number {
  return Math.max(0, Math.min(100, Math.round(n)));
}

function parseJsonArray(s: string | null): unknown[] {
  if (!s) return [];
  try {
    const v = JSON.parse(s);
    return Array.isArray(v) ? v : [];
  } catch {
    return [];
  }
}

function parseJsonObject(s: string): Record<string, unknown> {
  try {
    const v = JSON.parse(s);
    return v && typeof v === "object" && !Array.isArray(v) ? (v as Record<string, unknown>) : {};
  } catch {
    return {};
  }
}

async function readJson(c: { req: { json: () => Promise<unknown> } }): Promise<Record<string, unknown> | null> {
  try {
    const v = await c.req.json();
    return v && typeof v === "object" && !Array.isArray(v) ? (v as Record<string, unknown>) : null;
  } catch {
    return null;
  }
}
