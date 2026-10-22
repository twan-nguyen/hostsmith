import { describe, expect, it } from "vitest";

import type { HostEntry, HostsSnapshot, ListeningPort, Profile } from "./hosven";
import {
  activeTitle,
  blockRows,
  environmentRows,
  outcomeMessage,
  portTitle,
  portTrailing,
  userPorts,
} from "./menubar";

const profile = (id: string, name: string, isExclusive: boolean): Profile => ({
  id,
  name,
  color: isExclusive ? "amber" : "purple",
  isExclusive,
});

const entry = (tag: string, ip: string, hostname: string, enabled = true): HostEntry => ({
  id: `${tag}-${hostname}`,
  ip,
  hostname,
  aliases: [],
  comment: "",
  enabled,
  tag,
});

function snapshot(overrides: Partial<HostsSnapshot> = {}): HostsSnapshot {
  return {
    text: "",
    entries: [
      entry("Local", "127.0.0.1", "api.test"),
      entry("Staging", "10.0.0.2", "api.test", false),
      entry("Docker", "127.0.0.1", "db.test", false),
      entry("Mocks", "127.0.0.1", "a.test"),
      entry("Mocks", "127.0.0.1", "b.test", false),
    ],
    tags: [],
    profiles: [
      profile("local", "Local", true),
      profile("staging", "Staging", true),
      profile("empty", "Prod", true),
      profile("docker", "Docker", false),
      profile("mocks", "Mocks", false),
      profile("none", "Unused", false),
    ],
    active: { kind: "active", id: "local" },
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
    ...overrides,
  };
}

const port = (overrides: Partial<ListeningPort>): ListeningPort => ({
  id: "1",
  port: 3000,
  process: "node",
  pid: 1,
  user: "me",
  cwd: null,
  owner: { kind: "unknown" },
  isSystem: false,
  needsAttention: false,
  ...overrides,
});

describe("menu bar panel", () => {
  it("titles the card after the active environment", () => {
    expect(activeTitle(snapshot()).title).toBe("Local");
    expect(activeTitle(snapshot({ active: { kind: "none" } })).title).toBe("Không có môi trường");
    expect(activeTitle(snapshot({ active: { kind: "conflict", ids: ["local", "staging"] } })).title).toBe(
      "Nhiều môi trường đang bật",
    );
    expect(activeTitle(null).profile).toBeNull();
  });

  it("lists environments with entries, marking the active one", () => {
    expect(environmentRows(snapshot()).map((r) => [r.profile.name, r.entryCount, r.active])).toEqual([
      ["Local", 1, true],
      ["Staging", 1, false],
    ]);
  });

  it("marks every environment that is on while they conflict", () => {
    const conflict = snapshot({ active: { kind: "conflict", ids: ["local", "staging"] } });
    expect(environmentRows(conflict).map((r) => r.active)).toEqual([true, true]);
  });

  it("lists blocks with entries and their mapping", () => {
    expect(blockRows(snapshot()).map((r) => [r.profile.name, r.mapping, r.on])).toEqual([
      ["Docker", "127.0.0.1 → db.test", false],
      ["Mocks", "2 entry", true],
    ]);
  });

  it("shows ports the user started, with their owner or the call to stop them", () => {
    const ports = [
      port({ id: "a", owner: { kind: "yours", name: "api" } }),
      port({ id: "b", isSystem: true }),
      port({ id: "c", owner: { kind: "container", name: null } }),
      port({ id: "d", needsAttention: true }),
    ];
    const shown = userPorts({ ports, scannedAt: null, scanning: false, toast: null });
    expect(shown.map(portTrailing)).toEqual(["api", "docker", "Xem"]);
    expect(portTitle(ports[3])).toBe("3000  node · không rõ");
    expect(portTrailing(port({}))).toBe("me");
  });

  it("explains a switch that did not write", () => {
    expect(outcomeMessage({ kind: "refused", value: 2 })).toBe(
      "Có 2 thay đổi chưa áp dụng trong Hosven. Mở Hosven để áp dụng hoặc huỷ trước.",
    );
    expect(outcomeMessage({ kind: "failed", value: "denied" })).toBe("Không ghi được /etc/hosts: denied");
    expect(outcomeMessage({ kind: "written" })).toBeNull();
    expect(outcomeMessage({ kind: "cancelled" })).toBeNull();
  });
});
