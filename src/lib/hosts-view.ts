import type { Check, HostEntry, HostsSnapshot, Profile } from "@/lib/hostsmith";
import { plural } from "@/lib/utils";

// MARK: - Profiles

/** Environment and block profiles, with how many entries each holds and has on. */
export function profileStats(hosts: HostsSnapshot | null) {
  const entries = hosts?.entries ?? [];
  return (hosts?.profiles ?? []).map((profile) => {
    const own = entries.filter((e) => e.tag === profile.name);
    return {
      profile,
      entryCount: own.length,
      enabledCount: own.filter((e) => e.enabled).length,
      active:
        (hosts?.active.kind === "active" && hosts.active.id === profile.id) ||
        (hosts?.active.kind === "conflict" && hosts.active.ids.includes(profile.id)),
    };
  });
}

// MARK: - Resolver check

/** How many shown before the rest collapse into "…". */
const MISMATCH_LINES = 4;

export type VerificationSummary = { tone: "success" | "warning"; title: string; lines: string[] };

/** The resolver check after a write, as a counted fact and the hostnames that missed. */
export function verificationSummary(results: Check[]): VerificationSummary {
  const mismatches = results.filter((r) => !r.matches);
  if (mismatches.length === 0) {
    return {
      tone: "success",
      title: `${plural(results.length, "hostname")} resolved as written`,
      lines: ["Chrome keeps its own DNS cache, so the browser may point at the old address a while longer."],
    };
  }
  const lines = mismatches.slice(0, MISMATCH_LINES).map((r) => {
    const resolved = r.resolvedIps.length === 0 ? "unresolved" : r.resolvedIps.join(", ");
    return `${r.hostname} → ${resolved} (expected ${r.expectedIp})`;
  });
  if (mismatches.length > MISMATCH_LINES) lines.push("…");
  return {
    tone: "warning",
    title: `${mismatches.length} of ${plural(results.length, "hostname")} did not resolve as written in /etc/hosts`,
    lines,
  };
}

/** "Environment · 3 entries · 2 on". */
export function profileSubtitle(profile: Profile, entryCount: number, enabledCount: number): string {
  const parts = [profile.isExclusive ? "Environment" : "Block", plural(entryCount, "entry", "entries")];
  if (entryCount > 0) parts.push(`${enabledCount} on`);
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
  return [...names].filter(Boolean).sort((a, b) => a.localeCompare(b, "en", { sensitivity: "base" }));
}
