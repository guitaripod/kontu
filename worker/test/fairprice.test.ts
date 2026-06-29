import { describe, it, expect } from "vitest";
import { computeFairness, fairnessBand } from "../src/fairprice";

describe("fairnessBand", () => {
  it("maps ratios to the documented bands", () => {
    expect(fairnessBand(0.5)).toBe("underpriced");
    expect(fairnessBand(0.79)).toBe("underpriced");
    expect(fairnessBand(0.8)).toBe("below_market");
    expect(fairnessBand(0.9)).toBe("below_market");
    expect(fairnessBand(0.92)).toBe("fair");
    expect(fairnessBand(1.0)).toBe("fair");
    expect(fairnessBand(1.07)).toBe("fair");
    expect(fairnessBand(1.08)).toBe("above_market");
    expect(fairnessBand(1.19)).toBe("above_market");
    expect(fairnessBand(1.2)).toBe("overpriced");
    expect(fairnessBand(2.5)).toBe("overpriced");
  });

  it("returns 'unknown' for null/non-finite ratios", () => {
    expect(fairnessBand(null)).toBe("unknown");
    expect(fairnessBand(NaN)).toBe("unknown");
    expect(fairnessBand(Infinity)).toBe("unknown");
  });
});

describe("computeFairness", () => {
  const medians = new Map<string, number>([
    ["FI|helsinki", 335000],
    ["FI|outokumpu", 90000],
  ]);

  it("joins on a folded municipality name and computes the ratio", () => {
    const f = computeFairness(medians, "FI", "Helsinki", 360000);
    expect(f.benchmark).toBe(335000);
    expect(f.ratio).toBeCloseTo(360000 / 335000, 6);
    expect(f.band).toBe("fair");
    expect(f.confidence).toBe("medium");
  });

  it("folds diacritics and is case-insensitive", () => {
    const m = new Map<string, number>([["FI|jarvenpaa", 200000]]);
    const f = computeFairness(m, "FI", "Järvenpää", 230000);
    expect(f.benchmark).toBe(200000);
    expect(f.band).toBe("above_market");
  });

  it("returns unknown band/null benchmark when no median exists", () => {
    const f = computeFairness(medians, "FI", "Nokia", 150000);
    expect(f.benchmark).toBeNull();
    expect(f.ratio).toBeNull();
    expect(f.band).toBe("unknown");
    expect(f.confidence).toBe("unknown");
  });

  it("returns unknown when price is missing", () => {
    const f = computeFairness(medians, "FI", "Helsinki", null);
    expect(f.benchmark).toBe(335000);
    expect(f.ratio).toBeNull();
    expect(f.band).toBe("unknown");
    expect(f.confidence).toBe("medium");
  });
});
