import { describe, expect, it } from "vitest";

import { fuzzyMatch } from "./fuzzy";

describe("fuzzyMatch", () => {
  it("matches everything at a neutral score for an empty query", () => {
    expect(fuzzyMatch("", "anything")).toBe(0);
  });

  it("scores consecutive matches above scattered ones", () => {
    expect(fuzzyMatch("rel", "Release")!).toBeGreaterThan(fuzzyMatch("rel", "rXeXl")!);
  });

  it("prefers a match at the start of a word", () => {
    expect(fuzzyMatch("p", "Production")!).toBeGreaterThan(fuzzyMatch("p", "tap")!);
  });

  it("returns null when the query is not a subsequence", () => {
    expect(fuzzyMatch("xyz", "Release")).toBeNull();
  });

  it("rewards exact case", () => {
    expect(fuzzyMatch("Re", "Release")!).toBeGreaterThan(fuzzyMatch("re", "Release")!);
  });

  it("counts characters, not UTF-16 units", () => {
    // An emoji is one character, so both candidates are two long.
    expect(fuzzyMatch("b", "😀b")).toBe(fuzzyMatch("b", "-b"));
  });

  it("searches a thousand candidates in under 50 ms", () => {
    const candidates = Array.from({ length: 1000 }, (_, i) => `command-${i}-action-${i % 7}`);
    const start = performance.now();
    const hits = candidates.filter((c) => fuzzyMatch("act3", c) !== null).length;
    expect(performance.now() - start).toBeLessThan(50);
    expect(hits).toBeGreaterThan(0);
  });
});
