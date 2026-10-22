import { invoke } from "@tauri-apps/api/core";

import { profileStats } from "@/lib/hosts-view";
import type { HostsSnapshot, ListeningPort, Outcome, PortsSnapshot, Profile } from "@/lib/hostsmith";

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
  return profileStats(hosts).flatMap(({ profile, entryCount, enabledCount }) => {
    if (profile.isExclusive || entryCount === 0) return [];
    const only = entryCount === 1 ? hosts?.entries.find((e) => e.tag === profile.name) : undefined;
    const mapping = only ? `${only.ip} → ${only.hostname}` : `${entryCount} entry`;
    return [{ profile, mapping, on: enabledCount > 0 }];
  });
}

/** Environments a switch can turn on: those with entries. */
export function environmentRows(hosts: HostsSnapshot | null) {
  return profileStats(hosts).filter(({ profile, entryCount }) => profile.isExclusive && entryCount > 0);
}

/** Ports the user started, not the system's. */
export function userPorts(ports: PortsSnapshot | null): ListeningPort[] {
  return ports?.ports.filter((p) => !p.isSystem) ?? [];
}

export function portTitle(port: ListeningPort): string {
  return `${port.port}  ${port.process}${port.needsAttention ? " · không rõ" : ""}`;
}

/** Who a port belongs to, or a pointer to the Ports pane when nobody is known. */
export function portTrailing(port: ListeningPort): string {
  if (port.needsAttention) return "Xem";
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

function pendingMessage(count: number): string {
  return `Có ${count} thay đổi chưa áp dụng trong HostSmith. Mở HostSmith để áp dụng hoặc huỷ trước.`;
}
