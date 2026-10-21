import { describe, expect, it } from "vitest";

import { fuzzyMatch } from "./fuzzy";

describe("fuzzyMatch", () => {
  it("matches everything at a neutral score for an empty query", () => {
    expect(fuzzyMatch("", "anything")).toEqual({ score: 0, ranges: [] });
  });

  it("returns the runs of matched characters", () => {
    const m = fuzzyMatch("rls", "Release")!;
    expect(m.ranges).toEqual([
      [0, 1],
      [2, 3],
      [5, 6],
    ]);
  });

  it("scores consecutive matches above scattered ones", () => {
    expect(fuzzyMatch("rel", "Release")!.score).toBeGreaterThan(fuzzyMatch("rel", "rXeXl")!.score);
  });

  it("prefers a match at the start of a word", () => {
    expect(fuzzyMatch("p", "Production")!.score).toBeGreaterThan(fuzzyMatch("p", "tap")!.score);
  });

  it("returns null when the query is not a subsequence", () => {
    expect(fuzzyMatch("xyz", "Release")).toBeNull();
  });

  it("rewards exact case", () => {
    expect(fuzzyMatch("Re", "Release")!.score).toBeGreaterThan(fuzzyMatch("re", "Release")!.score);
  });

  it("counts characters, not UTF-16 units", () => {
    expect(fuzzyMatch("b", "😀b")!.ranges).toEqual([[1, 2]]);
  });

  it("searches a thousand candidates in under 50 ms", () => {
    const candidates = Array.from({ length: 1000 }, (_, i) => `command-${i}-action-${i % 7}`);
    const start = performance.now();
    const hits = candidates.filter((c) => fuzzyMatch("act3", c) !== null).length;
    expect(performance.now() - start).toBeLessThan(50);
    expect(hits).toBeGreaterThan(0);
  });
});
