import { describe, expect, it } from "vitest";

import { DEFAULT_SELECTION, formatSelection, parseSelection, type Selection } from "./selection";

describe("selection", () => {
  const cases: Selection[] = [
    { kind: "systemHosts" },
    { kind: "ports" },
    { kind: "settings" },
    { kind: "profile", id: "6f1c2a52-6d0e-4d43-9a2b-3c1f0e9d8b7a" },
    { kind: "hostsTag", name: "Docker" },
    { kind: "repo", id: "6f1c2a52-6d0e-4d43-9a2b-3c1f0e9d8b7a" },
    { kind: "envFile", repo: "6f1c2a52-6d0e-4d43-9a2b-3c1f0e9d8b7a", path: ".env.local" },
  ];

  it.each(cases)("round-trips %o", (selection) => {
    expect(parseSelection(formatSelection(selection))).toEqual(selection);
  });

  it("keeps a block name that contains a colon", () => {
    expect(parseSelection("hostsTag:api:v2")).toEqual({ kind: "hostsTag", name: "api:v2" });
  });

  it("falls back to the hosts file for anything unreadable", () => {
    for (const raw of [null, "", "nonsense", "profile:", "envFile:abc", "unknown:x"]) {
      expect(parseSelection(raw)).toEqual(DEFAULT_SELECTION);
    }
  });
});
