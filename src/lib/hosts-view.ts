import type { Check, HostEntry, HostsSnapshot, Profile, SwitchPreview, Toast } from "@/lib/hosven";

// MARK: - Banner slot

/** What the hosts pane's single banner slot shows, most urgent first. */
export type HostsBanner = "toast" | "conflict" | "loadError" | "externalChange" | "verification";

export function hostsBanner(hosts: HostsSnapshot | null, toast: Toast | null): HostsBanner | null {
  if (toast) return "toast";
  if (!hosts) return null;
  if (hosts.active.kind === "conflict") return "conflict";
  if (hosts.loadError) return "loadError";
  if (hosts.externalChange) return "externalChange";
  if (hosts.verification && hosts.verification.results.length > 0) {
    // A clean result only matters while what it checked is still what is on disk.
    const mismatched = hosts.verification.results.some((r) => !r.matches);
    if (mismatched || !hosts.dirty) return "verification";
  }
  return null;
}

/** What the profile pane's single banner slot shows, most urgent first. */
export type ProfileBanner = "toast" | "conflict" | "unsaved" | "active" | "activation";

export function profileBanner({
  hosts,
  toast,
  profile,
  entryCount,
  active,
}: {
  hosts: HostsSnapshot | null;
  toast: Toast | null;
  profile: Profile;
  entryCount: number;
  active: boolean;
}): ProfileBanner | null {
  if (toast) return "toast";
  if (!hosts) return null;
  if (profile.isExclusive && hosts.active.kind === "conflict") return "conflict";
  if (hosts.dirty) return "unsaved";
  if (active) return "active";
  if (profile.isExclusive && entryCount > 0) return "activation";
  return null;
}

// MARK: - Banner text

/** How many shown before the rest collapse into "…". */
export const MISMATCH_LINES = 4;

export type VerificationSummary = { tone: "success" | "warning"; title: string; lines: string[] };

/** The resolver check after a write, as a counted fact and the hostnames that missed. */
export function verificationSummary(results: Check[]): VerificationSummary {
  const mismatches = results.filter((r) => !r.matches);
  if (mismatches.length === 0) {
    return {
      tone: "success",
      title: `${results.length} hostname vừa ghi phân giải đúng`,
      lines: ["Chrome giữ cache DNS riêng, nên trình duyệt có thể vẫn trỏ về địa chỉ cũ thêm một lúc."],
    };
  }
  const lines = mismatches.slice(0, MISMATCH_LINES).map((r) => {
    const resolved = r.resolvedIps.length === 0 ? "không phân giải" : r.resolvedIps.join(", ");
    return `${r.hostname} → ${resolved} (mong đợi ${r.expectedIp})`;
  });
  if (mismatches.length > MISMATCH_LINES) lines.push("…");
  return {
    tone: "warning",
    title: `${mismatches.length} trong ${results.length} hostname vừa ghi chưa phân giải như /etc/hosts`,
    lines,
  };
}

/** "Kích hoạt sẽ bật 2 entry của Staging, tắt 3 entry của Local và ghi vào /etc/hosts" */
export function activationSummary(name: string, preview: SwitchPreview): string {
  const parts: string[] = [];
  if (preview.enable > 0) parts.push(`bật ${preview.enable} entry của ${name}`);
  for (const [other, count] of preview.disable) parts.push(`tắt ${count} entry của ${other}`);
  if (parts.length === 0) return `Kích hoạt sẽ đánh dấu ${name} là môi trường đang bật`;
  return `Kích hoạt sẽ ${parts.join(", ")} và ghi vào /etc/hosts`;
}

/** "Môi trường · 3 entry · ⌘2"; the shortcut belongs to environments only. */
export function profileSubtitle(profile: Profile, entryCount: number): string {
  const parts = [profile.isExclusive ? "Môi trường" : "Block", `${entryCount} entry`];
  const n = profile.shortcutNumber;
  if (profile.isExclusive && n !== undefined && n >= 1 && n <= 9) parts.push(`⌘${n}`);
  return parts.join(" · ");
}

// MARK: - Raw editor

/** The line that opens a block in the hosts file. */
export const blockMarker = (name: string) => `## [tag:${name}]`;

/** The draft to hand the model: nothing when the editor matches it again. */
export const draftFor = (text: string, modelText: string): string | null => (text === modelText ? null : text);

// MARK: - Entry form

/** Whether any entry already answers for `hostname`, aliases included, ignoring case. */
export function hostnameTaken(entries: HostEntry[], hostname: string, exceptId?: string): boolean {
  const wanted = hostname.trim().toLowerCase();
  if (!wanted) return false;
  return entries.some(
    (e) => e.id !== exceptId && [e.hostname, ...e.aliases].some((h) => h.toLowerCase() === wanted),
  );
}

/** Aliases typed space-separated, in order, without blanks. */
export const parseAliases = (text: string): string[] => text.split(/\s+/).filter(Boolean);

/** Every block an entry can join: existing tags, profiles without entries yet, and the preset one. */
export function blockChoices(tags: string[], profiles: Profile[], preset?: string | null): string[] {
  const names = new Set([...tags, ...profiles.map((p) => p.name), ...(preset ? [preset] : [])]);
  return [...names].filter(Boolean).sort((a, b) => a.localeCompare(b, "vi", { sensitivity: "base" }));
}
