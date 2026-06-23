import { describe, it, expect } from "vitest";
import { app } from "../src/index";

const env = { API_TOKEN: "secret" } as unknown as import("../src/index").Env;

describe("worker routes", () => {
  it("health returns ok", async () => {
    const res = await app.fetch(new Request("http://kontu/health"), env);
    expect(res.status).toBe(200);
    expect(await res.json()).toMatchObject({ ok: true, service: "kontu" });
  });

  it("rejects unauthenticated API calls", async () => {
    const res = await app.fetch(new Request("http://kontu/api/listings"), env);
    expect(res.status).toBe(401);
  });

  it("allows authenticated API calls", async () => {
    const res = await app.fetch(
      new Request("http://kontu/api/listings", {
        headers: { Authorization: "Bearer secret" },
      }),
      env,
    );
    expect(res.status).toBe(200);
  });
});
