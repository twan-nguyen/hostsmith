/**
 * What the Ports pane shows about each listening port. Pure, so filters,
 * labels and the scan age are tested without a window.
 */
import type { ListeningPort, PortOwner } from "@/lib/hostsmith";
import { abbreviateHome } from "@/lib/paths";
import type { PortsFilter } from "@/store";

/** "Dự án" shows ports a git project holds; "Người dùng" hides system services. */
export function filterPorts(ports: ListeningPort[], filter: PortsFilter): ListeningPort[] {
  switch (filter) {
    case "user":
      return ports.filter((p) => !p.isSystem);
    case "projects":
      return ports.filter((p) => p.owner.kind === "yours");
    case "system":
      return ports.filter((p) => p.isSystem);
  }
}

/** Ports a person started: every port that is not a system service. */
export const userPortCount = (ports: ListeningPort[]) => ports.filter((p) => !p.isSystem).length;

/** The first port no project or container owns, for the warning banner. */
export const attentionPort = (ports: ListeningPort[]) => ports.find((p) => p.needsAttention) ?? null;

/** Who holds the port: the project or container by name, else nobody known. */
export function ownerLabel(owner: PortOwner): string {
  switch (owner.kind) {
    case "yours":
      return owner.name;
    case "container":
      return owner.name ?? "Container";
    case "unknown":
      return "Không rõ";
  }
}

export type OwnerTone = "project" | "container" | "unknown";

export function ownerTone(port: ListeningPort): OwnerTone {
  return port.owner.kind === "yours" ? "project" : port.owner.kind === "container" ? "container" : "unknown";
}

/** The working folder worth showing: `/` says nothing about who started it. */
export const meaningfulCwd = (port: ListeningPort) => (port.cwd && port.cwd !== "/" ? port.cwd : null);

/** "pid N · ~/cwd"; the user it runs as when there is no folder worth showing. */
export function portDetail(port: ListeningPort, home: string | null): string {
  const cwd = meaningfulCwd(port);
  return [`pid ${port.pid}`, cwd ? abbreviateHome(cwd, home) : port.user].filter(Boolean).join(" · ");
}

/** How long ago the last scan finished: "vừa xong" under five seconds. */
export function scanAge(scannedAt: number | null, now: number): string | null {
  if (scannedAt === null) return null;
  const seconds = Math.max(0, Math.floor((now - scannedAt) / 1000));
  if (seconds < 5) return "vừa xong";
  if (seconds < 60) return `${seconds} giây trước`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes} phút trước`;
  return `${Math.floor(minutes / 60)} giờ trước`;
}

/** When the list was read: "quét vừa xong". */
export function portsSubtitle(scannedAt: number | null, now: number): string {
  const age = scanAge(scannedAt, now);
  return age ? `Quét ${age}` : "";
}

export const localhostUrl = (port: number) => `http://localhost:${port}`;
