/**
 * What the Ports pane shows about each listening port. Pure, so filters,
 * labels and the scan age are tested without a window.
 */
import type { ListeningPort, PortOwner } from "@/lib/hosven";
import { abbreviateHome } from "@/lib/paths";
import type { PortsFilter } from "@/store";

/** "Tất cả" hides system services; the other two narrow to one kind. */
export function filterPorts(ports: ListeningPort[], filter: PortsFilter): ListeningPort[] {
  switch (filter) {
    case "all":
      return ports.filter((p) => !p.isSystem);
    case "repos":
      return ports.filter((p) => p.owner.kind === "yours");
    case "system":
      return ports.filter((p) => p.isSystem);
  }
}

/** Ports a person started: every port that is not a system service. */
export const userPortCount = (ports: ListeningPort[]) => ports.filter((p) => !p.isSystem).length;

/** The first port nobody Hosven knows owns, for the warning banner. */
export const attentionPort = (ports: ListeningPort[]) => ports.find((p) => p.needsAttention) ?? null;

export function ownerLabel(owner: PortOwner): string {
  switch (owner.kind) {
    case "yours":
      return "Của bạn";
    case "container":
      return "Container";
    case "unknown":
      return "Không rõ";
  }
}

export type OwnerTone = "ok" | "idle" | "danger";

export function ownerTone(port: ListeningPort): OwnerTone {
  switch (port.owner.kind) {
    case "yours":
      return "ok";
    case "container":
      return "idle";
    case "unknown":
      return port.needsAttention ? "danger" : "idle";
  }
}

/** The working folder worth showing: `/` says nothing about who started it. */
export const meaningfulCwd = (port: ListeningPort) => (port.cwd && port.cwd !== "/" ? port.cwd : null);

/** "pid N · owner · ~/cwd": the repo, the container, or the user the process runs as. */
export function portDetail(port: ListeningPort, home: string | null): string {
  const owner =
    port.owner.kind === "yours" ? port.owner.name : port.owner.kind === "container" ? (port.owner.name ?? "docker") : port.user;
  const parts = [`pid ${port.pid}`, owner];
  const cwd = meaningfulCwd(port);
  if (cwd) parts.push(abbreviateHome(cwd, home));
  return parts.filter(Boolean).join(" · ");
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

/** The header subtitle: "N đang dùng · quét vừa xong". */
export function portsSubtitle(ports: ListeningPort[], scannedAt: number | null, now: number): string {
  const count = `${userPortCount(ports)} đang dùng`;
  const age = scanAge(scannedAt, now);
  return age ? `${count} · quét ${age}` : count;
}

export const localhostUrl = (port: number) => `http://localhost:${port}`;
