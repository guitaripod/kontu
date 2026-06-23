import { describe, it, expect } from "vitest";
import { buildListingsWhere } from "../src/db";

describe("buildListingsWhere", () => {
  it("returns no WHERE clause and no binds for an empty filter", () => {
    const q = buildListingsWhere({});
    expect(q.where).toBe("");
    expect(q.binds).toEqual([]);
  });

  it("builds municipality + price range with bound params in order", () => {
    const q = buildListingsWhere({ municipality: "Outokumpu", price_min: 50000, price_max: 200000 });
    expect(q.where).toContain("l.municipality = ? COLLATE NOCASE");
    expect(q.where).toContain("l.price_eur >= ?");
    expect(q.where).toContain("l.price_eur <= ?");
    expect(q.binds).toEqual(["Outokumpu", 50000, 200000]);
  });

  it("maps size, rooms, year, shore, heating and plot filters", () => {
    const q = buildListingsWhere({
      m2_min: 80,
      m2_max: 200,
      rooms_min: 3,
      year_min: 1980,
      shore: "oma_ranta",
      heating_type: "maalampo",
      plot_ownership: "oma",
    });
    expect(q.binds).toEqual([80, 200, 3, 1980, "oma_ranta", "maalampo", "oma"]);
  });

  it("translates energy_class_max into an A..G rank bound", () => {
    const q = buildListingsWhere({ energy_class_max: "C" });
    expect(q.where).toContain("instr('ABCDEFG', UPPER(l.energy_class))");
    expect(q.binds).toEqual([3]);
  });

  it("ignores an invalid energy_class_max", () => {
    const q = buildListingsWhere({ energy_class_max: "Z" });
    expect(q.where).toBe("");
    expect(q.binds).toEqual([]);
  });

  it("computes a first_seen cutoff for max_days_on_market", () => {
    const q = buildListingsWhere({ max_days_on_market: 14 });
    expect(q.where).toContain("l.first_seen >= ?");
    expect(q.binds).toHaveLength(1);
    const cutoff = q.binds[0] as number;
    const expected = Math.floor(Date.now() / 1000) - 14 * 86400;
    expect(Math.abs(cutoff - expected)).toBeLessThan(5);
  });

  it("adds three LIKE binds for free-text search", () => {
    const q = buildListingsWhere({ text: "ranta" });
    expect(q.where).toContain("l.address LIKE ?");
    expect(q.binds).toEqual(["%ranta%", "%ranta%", "%ranta%"]);
  });

  it("adds NOT LIKE binds per exclude keyword and skips empties", () => {
    const q = buildListingsWhere({ exclude: ["vuokra", "", "loma"] });
    expect(q.binds).toEqual(["%vuokra%", "%vuokra%", "%loma%", "%loma%"]);
  });

  it("adds a price_dropped EXISTS subquery without binds", () => {
    const q = buildListingsWhere({ price_dropped: true });
    expect(q.where).toContain("e.new_price_eur < e.old_price_eur");
    expect(q.binds).toEqual([]);
  });

  it("joins all clauses with AND and prefixes WHERE", () => {
    const q = buildListingsWhere({ municipality: "Joensuu", shore: "ei_rantaa" });
    expect(q.where.startsWith("WHERE ")).toBe(true);
    expect(q.where).toContain(" AND ");
  });

  it("keeps placeholder count equal to bind count", () => {
    const q = buildListingsWhere({
      municipality: "Kuopio",
      price_min: 100000,
      text: "sauna",
      exclude: ["vuokratontti"],
      energy_class_max: "D",
    });
    const placeholders = (q.where.match(/\?/g) ?? []).length;
    expect(placeholders).toBe(q.binds.length);
  });
});
