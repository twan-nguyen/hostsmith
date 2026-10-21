import { invoke } from "@tauri-apps/api/core";

import type { HostsSnapshot, ListeningPort, Outcome, PortsSnapshot, Profile } from "@/lib/hosven";

/** What the menu bar panel shows; the panel never scrolls, so lists are capped. */
export const MAX_TOGGLES = 8;
export const MAX_PORTS = 3;

export const panelToggleBlock = (name: string, enabled: boolean) =>
  invoke<Outcome>("panel_toggle_block", { name, enabled });
/** Shows the main window, on a sidebar selection when given. */
export const panelOpenMain = (selection: string | null = null) => invoke<void>("panel_open_main", { selection });
export const panelHide = () => invoke<void>("panel_hide");
export const panelQuit = () => invoke<void>("panel_quit");

/** The title of the active environment card. */
export function activeTitle(hosts: HostsSnapshot | null): { title: string; profile: Profile | null } {
  const active = hosts?.active;
  if (active?.kind === "active") {
    const profile = hosts?.profiles.find((p) => p.id === active.id) ?? null;
    if (profile) return { title: profile.name, profile };
  }
  if (active?.kind === "conflict") return { title: "Nhiều môi trường đang bật", profile: null };
  return { title: "Không có môi trường", profile: null };
}

export type BlockRow = { profile: Profile; mapping: string; on: boolean };

/** Blocks with entries, each with its mapping and whether any entry is on. */
export function blockRows(hosts: HostsSnapshot | null): BlockRow[] {
  if (!hosts) return [];
  return hosts.profiles.flatMap((profile) => {
    if (profile.isExclusive) return [];
    const own = hosts.entries.filter((e) => e.tag === profile.name);
    if (own.length === 0) return [];
    const mapping = own.length === 1 ? `${own[0].ip} → ${own[0].hostname}` : `${own.length} entry`;
    return [{ profile, mapping, on: own.some((e) => e.enabled) }];
  });
}

export type EnvironmentRow = { profile: Profile; entryCount: number; active: boolean };

/** Environments a switch can turn on: those with entries. */
export function environmentRows(hosts: HostsSnapshot | null): EnvironmentRow[] {
  if (!hosts) return [];
  return hosts.profiles.flatMap((profile) => {
    if (!profile.isExclusive) return [];
    const entryCount = hosts.entries.filter((e) => e.tag === profile.name).length;
    if (entryCount === 0) return [];
    return [{ profile, entryCount, active: hosts.active.kind === "active" && hosts.active.id === profile.id }];
  });
}

/** Ports the user started, not the system's. */
export function userPorts(ports: PortsSnapshot | null): ListeningPort[] {
  return ports?.ports.filter((p) => !p.isSystem) ?? [];
}

export function portTitle(port: ListeningPort): string {
  return `${port.port}  ${port.process}${port.needsAttention ? " · không rõ" : ""}`;
}

/** Who a port belongs to, or the call to stop it when nobody is known. */
export function portTrailing(port: ListeningPort): string {
  if (port.needsAttention) return "Dừng";
  switch (port.owner.kind) {
    case "yours":
      return port.owner.name;
    case "container":
      return port.owner.name ?? "docker";
    case "unknown":
      return port.user;
  }
}

/** What the panel says when a switch or a flip did not write. */
export function outcomeMessage(outcome: Outcome): string | null {
  switch (outcome.kind) {
    case "refused":
      return pendingMessage(outcome.value);
    case "failed":
      return `Không ghi được /etc/hosts: ${outcome.value}`;
    default:
      return null;
  }
}

export function pendingMessage(count: number): string {
  return `Có ${count} thay đổi chưa áp dụng trong Hosven. Mở Hosven để áp dụng hoặc huỷ trước.`;
}
