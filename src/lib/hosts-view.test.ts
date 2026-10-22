import { describe, expect, it } from "vitest";

import type { Check, HostEntry, HostsSnapshot, Profile } from "./hostsmith";
import {
  blockChoices,
  blockMarker,
  draftFor,
  hostnameTaken,
  parseAliases,
  profileStats,
  profileSubtitle,
  verificationSummary,
} from "./hosts-view";

function snapshot(patch: Partial<HostsSnapshot> = {}): HostsSnapshot {
  return {
    text: "127.0.0.1\tlocalhost\n",
    entries: [],
    tags: [],
    profiles: [],
    active: { kind: "none" },
    dirty: false,
    pendingCount: 0,
    canUndo: false,
    canRedo: false,
    applying: false,
    externalChange: false,
    loadError: null,
    verification: null,
    pendingSwitch: null,
    toast: null,
    ...patch,
  };
}

const ok: Check = { hostname: "api.test", expectedIp: "127.0.0.1", resolvedIps: ["127.0.0.1"], matches: true };
const miss = (hostname: string, resolvedIps: string[] = []): Check => ({
  hostname,
  expectedIp: "127.0.0.1",
  resolvedIps,
  matches: false,
});

const env: Profile = { id: "e1", name: "Staging", color: "amber", shortcutNumber: 2, isExclusive: true };
const block: Profile = { id: "b1", name: "Docker", color: "purple", shortcutNumber: 3, isExclusive: false };

describe("profileStats", () => {
  const entry = (tag: string, enabled: boolean): HostEntry => ({
    id: `${tag}-${enabled}`,
    ip: "127.0.0.1",
    hostname: `${tag}.test`,
    aliases: [],
    comment: "",
    enabled,
    tag,
  });

  it("counts each profile's entries and marks every environment that is on", () => {
    const hosts = snapshot({
      profiles: [env, block, { ...env, id: "e2", name: "Local" }],
      entries: [entry("Staging", true), entry("Docker", true), entry("Docker", false)],
      active: { kind: "conflict", ids: ["e1", "e2"] },
    });
    expect(profileStats(hosts).map((s) => [s.profile.name, s.entryCount, s.enabledCount, s.active])).toEqual([
      ["Staging", 1, 1, true],
      ["Docker", 2, 1, false],
      ["Local", 0, 0, true],
    ]);
    expect(profileStats(null)).toEqual([]);
  });
});

describe("verificationSummary", () => {
  it("counts the hostnames that resolved", () => {
    const summary = verificationSummary([ok, { ...ok, hostname: "web.test" }]);
    expect(summary.tone).toBe("success");
    expect(summary.title).toBe("2 hostnames resolved as written");
    expect(verificationSummary([ok]).title).toBe("1 hostname resolved as written");
  });

  it("lists up to four mismatches, then an ellipsis", () => {
    const results = [ok, miss("a.test", ["10.0.0.1", "10.0.0.2"]), miss("b.test"), miss("c.test"), miss("d.test"), miss("e.test")];
    const summary = verificationSummary(results);
    expect(summary.tone).toBe("warning");
    expect(summary.title).toBe("5 of 6 hostnames did not resolve as written in /etc/hosts");
    expect(summary.lines).toEqual([
      "a.test → 10.0.0.1, 10.0.0.2 (expected 127.0.0.1)",
      "b.test → unresolved (expected 127.0.0.1)",
      "c.test → unresolved (expected 127.0.0.1)",
      "d.test → unresolved (expected 127.0.0.1)",
      "…",
    ]);
  });

  it("has no ellipsis at exactly four", () => {
    const summary = verificationSummary([miss("a"), miss("b"), miss("c"), miss("d")]);
    expect(summary.lines).toHaveLength(4);
  });
});

describe("profileSubtitle", () => {
  it("names the kind, then counts entries and how many are on", () => {
    expect(profileSubtitle(env, 3, 2)).toBe("Environment · 3 entries · 2 on");
    expect(profileSubtitle(block, 1, 0)).toBe("Block · 1 entry · 0 on");
    expect(profileSubtitle(env, 0, 0)).toBe("Environment · 0 entries");
  });
});

describe("raw editor helpers", () => {
  it("builds the block marker", () => {
    expect(blockMarker("Docker")).toBe("## [tag:Docker]");
  });

  it("sends no draft when the editor matches the model", () => {
    expect(draftFor("a\n", "a\n")).toBeNull();
    expect(draftFor("a\nb\n", "a\n")).toBe("a\nb\n");
  });
});

describe("entry form helpers", () => {
  const entry = (id: string, hostname: string, aliases: string[] = []): HostEntry => ({
    id,
    ip: "127.0.0.1",
    hostname,
    aliases,
    comment: "",
    enabled: true,
    tag: null,
  });
  const entries = [entry("1", "api.test", ["www.api.test"]), entry("2", "web.test")];

  it("finds a hostname among names and aliases, ignoring case", () => {
    expect(hostnameTaken(entries, "API.test")).toBe(true);
    expect(hostnameTaken(entries, " www.api.test ")).toBe(true);
    expect(hostnameTaken(entries, "new.test")).toBe(false);
    expect(hostnameTaken(entries, "")).toBe(false);
  });

  it("skips the entry being edited", () => {
    expect(hostnameTaken(entries, "api.test", "1")).toBe(false);
    expect(hostnameTaken(entries, "web.test", "1")).toBe(true);
  });

  it("splits aliases on whitespace", () => {
    expect(parseAliases("  a.test   b.test\tc.test ")).toEqual(["a.test", "b.test", "c.test"]);
    expect(parseAliases("   ")).toEqual([]);
  });

  it("offers each block once, sorted", () => {
    expect(blockChoices(["Docker", "Local"], [env, block], "Api")).toEqual(["Api", "Docker", "Local", "Staging"]);
    expect(blockChoices([], [], null)).toEqual([]);
  });
});
