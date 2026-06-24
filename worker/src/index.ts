import { Hono } from "hono";
import { api } from "./api/listings";
import { crawlTick } from "./crawl";
import { enrichBatch } from "./geo";

export interface Env {
  DB: D1Database;
  PHOTOS: R2Bucket;
  API_TOKEN: string;
  DIGITRANSIT_KEY?: string;
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

app.use("/api/*", async (c, next) => {
  const token = c.req.header("Authorization")?.replace(/^Bearer\s+/i, "");
  if (!token || token !== c.env.API_TOKEN) {
    return c.json({ error: "unauthorized" }, 401);
  }
  await next();
});

app.get("/api/photos/:key{.+}", async (c) => {
  const key = c.req.param("key");
  const object = await c.env.PHOTOS.get(key);
  if (!object) return c.json({ error: "not found" }, 404);
  const headers = new Headers();
  object.writeHttpMetadata(headers);
  headers.set("etag", object.httpEtag);
  headers.set("Cache-Control", "public, max-age=31536000, immutable");
  return new Response(object.body, { headers });
});

app.route("/api", api);

export default {
  fetch: app.fetch,
  async scheduled(_controller: ScheduledController, env: Env, ctx: ExecutionContext): Promise<void> {
    ctx.waitUntil(runScheduled(env));
  },
} satisfies ExportedHandler<Env>;

async function runScheduled(env: Env): Promise<void> {
  try {
    await crawlTick(env.DB, env.PHOTOS);
  } catch (err) {
    console.warn("scheduled crawl tick failed", String(err));
  }
  try {
    await enrichBatch(env.DB, 5);
  } catch (err) {
    console.warn("scheduled enrichment failed", String(err));
  }
}
