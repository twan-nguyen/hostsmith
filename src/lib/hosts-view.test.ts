import { describe, expect, it } from "vitest";

import type { Check, HostEntry, HostsSnapshot, Profile } from "./hosven";
import {
  activationSummary,
  blockChoices,
  blockMarker,
  draftFor,
  hostnameTaken,
  hostsBanner,
  parseAliases,
  profileBanner,
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

describe("hostsBanner", () => {
  it("ranks conflict over a load error over an outside change over verification", () => {
    const all = snapshot({
      active: { kind: "conflict", ids: ["a", "b"] },
      loadError: "denied",
      externalChange: true,
      verification: { results: [ok] },
    });
    expect(hostsBanner(all)).toBe("conflict");
    expect(hostsBanner({ ...all, active: { kind: "none" } })).toBe("loadError");
    expect(hostsBanner({ ...all, active: { kind: "none" }, loadError: null })).toBe("externalChange");
    expect(hostsBanner({ ...all, active: { kind: "none" }, loadError: null, externalChange: false })).toBe(
      "verification",
    );
  });

  it("hides a clean verification once new edits are pending, but keeps a mismatch", () => {
    expect(hostsBanner(snapshot({ dirty: true, verification: { results: [ok] } }))).toBeNull();
    expect(hostsBanner(snapshot({ dirty: true, verification: { results: [ok, miss("b.test")] } }))).toBe(
      "verification",
    );
  });

  it("shows nothing without a snapshot or a reason", () => {
    expect(hostsBanner(null)).toBeNull();
    expect(hostsBanner(snapshot())).toBeNull();
  });
});

describe("profileBanner", () => {
  const base = { hosts: snapshot(), profile: env, entryCount: 2, active: false };

  it("puts a conflict before the activation summary", () => {
    const conflict = snapshot({ active: { kind: "conflict", ids: ["e1", "e2"] }, dirty: true });
    expect(profileBanner({ ...base, hosts: conflict })).toBe("conflict");
    expect(profileBanner(base)).toBe("activation");
  });

  it("leaves pending edits and the on state to the header", () => {
    expect(profileBanner({ ...base, hosts: snapshot({ dirty: true }) })).toBeNull();
    expect(profileBanner({ ...base, active: true })).toBeNull();
  });

  it("leaves the conflict to environments and the activation summary too", () => {
    const conflict = snapshot({ active: { kind: "conflict", ids: ["e1", "e2"] } });
    expect(profileBanner({ ...base, profile: block, hosts: conflict })).toBeNull();
    expect(profileBanner({ ...base, profile: block })).toBeNull();
  });

  it("offers no activation summary for an empty environment", () => {
    expect(profileBanner({ ...base, entryCount: 0 })).toBeNull();
  });
});

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
    expect(summary.title).toBe("2 hostname vừa ghi phân giải đúng");
  });

  it("lists up to four mismatches, then an ellipsis", () => {
    const results = [ok, miss("a.test", ["10.0.0.1", "10.0.0.2"]), miss("b.test"), miss("c.test"), miss("d.test"), miss("e.test")];
    const summary = verificationSummary(results);
    expect(summary.tone).toBe("warning");
    expect(summary.title).toBe("5 trong 6 hostname vừa ghi chưa phân giải như /etc/hosts");
    expect(summary.lines).toEqual([
      "a.test → 10.0.0.1, 10.0.0.2 (mong đợi 127.0.0.1)",
      "b.test → không phân giải (mong đợi 127.0.0.1)",
      "c.test → không phân giải (mong đợi 127.0.0.1)",
      "d.test → không phân giải (mong đợi 127.0.0.1)",
      "…",
    ]);
  });

  it("has no ellipsis at exactly four", () => {
    const summary = verificationSummary([miss("a"), miss("b"), miss("c"), miss("d")]);
    expect(summary.lines).toHaveLength(4);
  });
});

describe("activationSummary", () => {
  it("names what turns on and off", () => {
    expect(activationSummary("Staging", { enable: 2, disable: [["Local", 3]] })).toBe(
      "Kích hoạt sẽ bật 2 entry của Staging, tắt 3 entry của Local và ghi vào /etc/hosts",
    );
  });

  it("says so when nothing changes", () => {
    expect(activationSummary("Staging", { enable: 0, disable: [] })).toBe(
      "Kích hoạt sẽ đánh dấu Staging là môi trường đang bật",
    );
  });
});

describe("profileSubtitle", () => {
  it("shows the shortcut for environments only", () => {
    expect(profileSubtitle(env, 3)).toBe("Môi trường · 3 entry · ⌘2");
    expect(profileSubtitle(block, 1)).toBe("Block · 1 entry");
    expect(profileSubtitle({ ...env, shortcutNumber: undefined }, 0)).toBe("Môi trường · 0 entry");
    expect(profileSubtitle({ ...env, shortcutNumber: 12 }, 0)).toBe("Môi trường · 0 entry");
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
