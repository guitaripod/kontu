import { Hono } from "hono";
import { api } from "./api/listings";
import { crawlTick } from "./crawl";
import { enrichBatch, enrichShoreBatch } from "./geo";
import { marketIsStale, refreshMarketStats, refreshNordicMarketStats } from "./fairprice";
import { getPhotoSourceByKey, getPublishedPage, listPublishedPages } from "./db";
import { renderIndexPage, renderListingPage, type PublishedPayload } from "./page";

export interface Env {
  DB: D1Database;
  PHOTOS: R2Bucket;
  API_TOKEN: string;
  DIGITRANSIT_KEY?: string;
  MML_API_KEY?: string;
}

export const app = new Hono<{ Bindings: Env }>();

app.get("/", (c) =>
  c.html(
    `<!doctype html><meta charset=utf8><title>kontu</title>
<style>body{background:#16181d;color:#d8d8d8;font:15px/1.6 ui-monospace,monospace;max-width:42rem;margin:12vh auto;padding:0 1.5rem}a{color:#6cb6ff}code{color:#9ad8ff}</style>
<h1>kontu</h1>
<p>Private API backing the <em>kontu</em> Finnish house-hunting TUI &amp; CLI. Not a website.</p>
<p>Listing/history/cost data lives behind a bearer token; the only public endpoint is
<a href="/health"><code>/health</code></a>. Use the <code>kontu</code> CLI/TUI, not a browser.</p>`,
  ),
);

app.get("/health", (c) =>
  c.json({ ok: true, service: "kontu", ts: new Date().toISOString() }),
);

const FAVICON =
  '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"><rect x="0" y="0" width="32" height="32" rx="6" fill="#14180f"/><path d="M16 4 L28 14 L25 14 L25 27 L7 27 L7 14 L4 14 Z" fill="#3fae6f"/><path d="M10 18 L14.5 22.5 L23 12.5" fill="none" stroke="#efe7d2" stroke-width="3.8" stroke-linecap="round" stroke-linejoin="round"/></svg>';

app.get("/favicon.svg", (c) =>
  c.body(FAVICON, 200, { "Content-Type": "image/svg+xml", "Cache-Control": "public, max-age=604800" }),
);
app.get("/favicon.ico", (c) => c.redirect("/favicon.svg", 301));

app.get("/kontu", async (c) => {
  const rows = await listPublishedPages(c.env.DB);
  const items: PublishedPayload[] = [];
  for (const r of rows) {
    try {
      items.push(JSON.parse(r) as PublishedPayload);
    } catch {
      /* skip a corrupt row */
    }
  }
  // "Scanned" counts only listings we actually read (detail-enriched). Card-only
  // listings (Etuovi, whose detail API is gated) have no description and can't be
  // evaluated, so counting them would overstate the scan.
  const stat = await c.env.DB.prepare(
    "SELECT " +
      "SUM(CASE WHEN description IS NOT NULL AND description != '' THEN 1 ELSE 0 END) AS scanned, " +
      "COUNT(DISTINCT CASE WHEN description IS NOT NULL AND description != '' THEN municipality END) AS municipalities, " +
      "SUM(CASE WHEN description IS NULL OR description = '' THEN 1 ELSE 0 END) AS unread, " +
      "MAX(last_seen) AS updated FROM listings",
  ).first<{ scanned: number; municipalities: number; unread: number; updated: number | null }>();
  const origin = new URL(c.req.url).origin;
  const market = {
    scanned: stat?.scanned ?? items.length,
    municipalities: stat?.municipalities ?? 0,
    unread: stat?.unread ?? 0,
    updated: stat?.updated ?? null,
  };
  return c.html(renderIndexPage(items, origin, market), 200, {
    "Cache-Control": "public, max-age=300",
    "X-Robots-Tag": "noindex, nofollow",
  });
});

app.get("/kontu/:id", async (c) => {
  const id = Number(c.req.param("id"));
  if (!Number.isInteger(id)) return c.text("not found", 404);
  const payload = await getPublishedPage(c.env.DB, id);
  if (!payload) return c.text("not found", 404);
  let data: PublishedPayload;
  try {
    data = JSON.parse(payload) as PublishedPayload;
  } catch {
    return c.text("page unavailable", 500);
  }
  const origin = new URL(c.req.url).origin;
  return c.html(renderListingPage(data, origin), 200, {
    "Cache-Control": "public, max-age=600",
    "X-Robots-Tag": "noindex, nofollow",
  });
});

app.get("/h", (c) => c.redirect("/kontu", 301));
app.get("/h/:id", (c) => c.redirect(`/kontu/${c.req.param("id")}`, 301));

app.use("/api/*", async (c, next) => {
  // Cover photos are public images — the public site embeds them as <img src>, which
  // can't carry a bearer token. They expose nothing private (the same image is public
  // on the portal), so the photo route is exempt from the write-API token guard.
  if (c.req.path.startsWith("/api/photos/")) return next();
  const token = c.req.header("Authorization")?.replace(/^Bearer\s+/i, "");
  if (!token || token !== c.env.API_TOKEN) {
    return c.json({ error: "unauthorized" }, 401);
  }
  await next();
});

app.get("/api/photos/:key{.+}", async (c) => {
  const key = c.req.param("key");
  let object = await c.env.PHOTOS.get(key);
  if (!object) object = await readThroughPhoto(c.env, key);
  if (!object) return c.json({ error: "not found" }, 404);
  const headers = new Headers();
  object.writeHttpMetadata(headers);
  headers.set("etag", object.httpEtag);
  headers.set("Cache-Control", "public, max-age=31536000, immutable");
  return new Response(object.body, { headers });
});

/**
 * Lazily populate R2 on a cache miss: cover photos are recorded at import time as
 * a URL-derived key + source URL only (no download). On first view we fetch the
 * source, store it, and serve it — so imports stay fast and only viewed images
 * are ever pulled. Returns the stored object, or null if it can't be fetched.
 */
const MAX_PHOTO_BYTES = 8_000_000;

/** Reject non-https and host shapes that could reach internal/metadata endpoints
 *  (literal IPs, localhost, *.internal/*.local) — legit photo CDNs use hostnames. */
function isUnsafePhotoUrl(src: string): boolean {
  let u: URL;
  try {
    u = new URL(src);
  } catch {
    return true;
  }
  if (u.protocol !== "https:") return true;
  const h = u.hostname.toLowerCase();
  return (
    h === "localhost" ||
    h.endsWith(".internal") ||
    h.endsWith(".local") ||
    h.includes(":") ||
    /^\d{1,3}(\.\d{1,3}){3}$/.test(h)
  );
}

async function readThroughPhoto(env: Env, key: string): Promise<R2ObjectBody | null> {
  const src = await getPhotoSourceByKey(env.DB, key);
  if (!src || isUnsafePhotoUrl(src)) return null;
  try {
    const res = await fetch(src);
    if (!res.ok) return null;
    const contentType = res.headers.get("content-type") ?? "";
    if (!contentType.startsWith("image/")) return null;
    const bytes = await res.arrayBuffer();
    if (bytes.byteLength === 0 || bytes.byteLength > MAX_PHOTO_BYTES) return null;
    await env.PHOTOS.put(key, bytes, { httpMetadata: { contentType } });
    return await env.PHOTOS.get(key);
  } catch {
    return null;
  }
}

app.route("/api", api);

export default {
  fetch: app.fetch,
  async scheduled(_controller: ScheduledController, env: Env, ctx: ExecutionContext): Promise<void> {
    ctx.waitUntil(runScheduled(env));
  },
} satisfies ExportedHandler<Env>;

async function runScheduled(env: Env): Promise<void> {
  // Drain a few crawl pages of the open-API portals (DK Boligsiden, IS visir) per run.
  // Bounded: each tick fetches one page + up to MAX_PHOTOS_PER_TICK photos, and Cloudflare
  // caps subrequests per invocation — 10 ticks × 24 photos blew that cap and wedged whole
  // DK regions / IS postcodes in a permanent error state. 3 ticks keeps us well under while
  // still draining in days; the cron runs daily so it catches up. Stops early when idle.
  for (let i = 0; i < 3; i++) {
    try {
      const r = await crawlTick(env.DB, env.PHOTOS);
      if (!r.source) break;
    } catch (err) {
      console.warn("scheduled crawl tick failed", String(err));
      break;
    }
  }
  try {
    await enrichBatch(env.DB, 8);
  } catch (err) {
    console.warn("scheduled enrichment failed", String(err));
  }
  try {
    // Country-agnostic geometric shore detection (OSM) — the automated signal that
    // lets a lakeside match anywhere in the Nordics be detected without a manual pass.
    await enrichShoreBatch(env.DB, 25);
  } catch (err) {
    console.warn("scheduled shore enrichment failed", String(err));
  }
  try {
    if (await marketIsStale(env.DB)) await refreshMarketStats(env.DB);
  } catch (err) {
    console.warn("scheduled market refresh failed", String(err));
  }
  try {
    await refreshNordicMarketStats(env.DB);
  } catch (err) {
    console.warn("scheduled nordic market refresh failed", String(err));
  }
}
