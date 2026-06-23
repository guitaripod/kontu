import { Hono } from "hono";

export interface Env {
  DB: D1Database;
  PHOTOS: R2Bucket;
  API_TOKEN: string;
}

export const app = new Hono<{ Bindings: Env }>();

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

app.get("/api/listings", (c) => c.json({ listings: [] }));

export default {
  fetch: app.fetch,
} satisfies ExportedHandler<Env>;
