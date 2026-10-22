import { describe, expect, it } from "vitest";

import type { Check, HostsSnapshot } from "./hostsmith";
import { cleanVerification, hostsAlerts, noticeMs } from "./notices";

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
const miss: Check = { hostname: "web.test", expectedIp: "127.0.0.1", resolvedIps: [], matches: false };

describe("hostsAlerts", () => {
  it("lists nothing for a healthy file", () => {
    expect(hostsAlerts(null)).toEqual([]);
    expect(hostsAlerts(snapshot({ verification: { results: [ok] } }))).toEqual([]);
  });

  it("orders standing problems by urgency", () => {
    const hosts = snapshot({
      loadError: "denied",
      externalChange: true,
      active: { kind: "conflict", ids: ["a", "b"] },
      verification: { results: [ok, miss] },
    });
    expect(hostsAlerts(hosts)).toEqual(["loadError", "externalChange", "conflict", "mismatch"]);
  });
});

describe("cleanVerification", () => {
  it("holds only when every checked hostname answered as written", () => {
    expect(cleanVerification(snapshot({ verification: { results: [ok] } }))).toBe(true);
    expect(cleanVerification(snapshot({ verification: { results: [ok, miss] } }))).toBe(false);
    expect(cleanVerification(snapshot({ verification: { results: [] } }))).toBe(false);
    expect(cleanVerification(snapshot())).toBe(false);
  });
});

describe("noticeMs", () => {
  it("gives errors longer to be read", () => {
    expect(noticeMs("danger")).toBeGreaterThan(noticeMs("success"));
  });
});
