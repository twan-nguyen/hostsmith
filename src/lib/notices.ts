import type { HostsSnapshot, ToastKind } from "@/lib/hostsmith";

export type NoticeTone = "success" | "info" | "warning" | "danger";

export const TOAST_TONE: Record<ToastKind, NoticeTone> = { success: "success", error: "danger", info: "info" };

/** How long a passing notice stays up; an error gets longer to be read. */
export function noticeMs(tone: NoticeTone): number {
  return tone === "danger" ? 6000 : 3500;
}

/**
 * States of the hosts file that stay on screen until they are resolved, most
 * urgent first. Each blocks or questions the next write, so none of them is a
 * passing message.
 */
export type HostsAlert = "loadError" | "externalChange" | "conflict" | "mismatch";

export function hostsAlerts(hosts: HostsSnapshot | null): HostsAlert[] {
  if (!hosts) return [];
  const alerts: HostsAlert[] = [];
  if (hosts.loadError) alerts.push("loadError");
  if (hosts.externalChange) alerts.push("externalChange");
  if (hosts.active.kind === "conflict") alerts.push("conflict");
  if (hosts.verification?.results.some((r) => !r.matches)) alerts.push("mismatch");
  return alerts;
}

/** A resolver check where every hostname answered as written: worth a passing note, nothing more. */
export function cleanVerification(hosts: HostsSnapshot | null): boolean {
  const results = hosts?.verification?.results ?? [];
  return results.length > 0 && results.every((r) => r.matches);
}
