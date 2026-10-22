import { describe, expect, it } from "vitest";

import type { ListeningPort } from "@/lib/hostsmith";
import { abbreviateHome } from "@/lib/paths";
import {
  attentionPort,
  filterPorts,
  localhostUrl,
  ownerLabel,
  ownerTone,
  portDetail,
  portsSubtitle,
  scanAge,
} from "@/lib/ports-view";

function port(overrides: Partial<ListeningPort>): ListeningPort {
  const base: ListeningPort = {
    id: "1-1",
    port: 3000,
    process: "node",
    pid: 100,
    user: "tuan",
    cwd: "/Users/tuan/api",
    owner: { kind: "unknown" },
    isSystem: false,
    needsAttention: false,
  };
  return { ...base, ...overrides, id: `${overrides.port ?? base.port}-${overrides.pid ?? base.pid}` };
}

const yours = port({ port: 3000, owner: { kind: "yours", name: "api" } });
const container = port({ port: 5432, process: "com.docker.backend", owner: { kind: "container", name: "db" } });
const stranger = port({ port: 8080, process: "python3", pid: 200, needsAttention: true });
const system = port({ port: 5000, process: "ControlCenter", pid: 300, isSystem: true, cwd: "/" });
const all = [yours, container, stranger, system];

describe("filters", () => {
  it("shows everything but system services under User", () => {
    expect(filterPorts(all, "user")).toEqual([yours, container, stranger]);
  });

  it("narrows to project ports and to system services", () => {
    expect(filterPorts(all, "projects")).toEqual([yours]);
    expect(filterPorts(all, "system")).toEqual([system]);
  });

  it("finds the first port that needs attention", () => {
    expect(attentionPort(all)).toBe(stranger);
    expect(attentionPort([yours, system])).toBeNull();
  });
});

describe("labels", () => {
  it("names each owner", () => {
    expect(ownerLabel(yours.owner)).toBe("api");
    expect(ownerLabel(container.owner)).toBe("db");
    expect(ownerLabel({ kind: "container", name: null })).toBe("Container");
    expect(ownerLabel(stranger.owner)).toBe("Unknown");
  });

  it("marks only unknown ports that need attention as danger", () => {
    expect(ownerTone(yours)).toBe("project");
    expect(ownerTone(container)).toBe("container");
    expect(ownerTone(stranger)).toBe("unknown");
  });

  it("describes the pid, the owner and the folder under home", () => {
    const home = "/Users/tuan";
    expect(portDetail(yours, home)).toBe("pid 100 · ~/api");
    expect(portDetail(port({ owner: { kind: "container", name: null }, cwd: null }), home)).toBe("pid 100 · tuan");
    expect(portDetail(stranger, home)).toBe("pid 200 · ~/api");
    expect(portDetail(system, home)).toBe("pid 300 · tuan");
  });

  it("opens the port on localhost", () => {
    expect(localhostUrl(5173)).toBe("http://localhost:5173");
  });
});

describe("scan age", () => {
  const scannedAt = 1_000_000;

  it("says nothing before the first scan", () => {
    expect(scanAge(null, scannedAt)).toBeNull();
    expect(portsSubtitle(null, scannedAt)).toBe("");
  });

  it("says just now under five seconds, then counts seconds", () => {
    expect(scanAge(scannedAt, scannedAt + 4_999)).toBe("just now");
    expect(scanAge(scannedAt, scannedAt + 5_000)).toBe("5 sec ago");
    expect(scanAge(scannedAt, scannedAt + 59_000)).toBe("59 sec ago");
  });

  it("switches to minutes and hours for old scans", () => {
    expect(scanAge(scannedAt, scannedAt + 125_000)).toBe("2 min ago");
    expect(scanAge(scannedAt, scannedAt + 2 * 3_600_000)).toBe("2 hr ago");
  });

  it("treats a clock that went back as just now", () => {
    expect(scanAge(scannedAt, scannedAt - 10_000)).toBe("just now");
  });

  it("builds the subtitle from the non-system count", () => {
    expect(portsSubtitle(scannedAt, scannedAt + 1_000)).toBe("Scanned just now");
    expect(portsSubtitle(scannedAt, scannedAt + 12_000)).toBe("Scanned 12 sec ago");
  });
});

describe("paths", () => {
  it("abbreviates the home folder only at a path boundary", () => {
    expect(abbreviateHome("/Users/tuan/api/src", "/Users/tuan")).toBe("~/api/src");
    expect(abbreviateHome("/Users/tuan", "/Users/tuan/")).toBe("~");
    expect(abbreviateHome("/Users/tuanx/api", "/Users/tuan")).toBe("/Users/tuanx/api");
    expect(abbreviateHome("/opt/api", null)).toBe("/opt/api");
  });
});
