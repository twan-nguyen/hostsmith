/**
 * What the sidebar points at. One value drives the whole working pane, and
 * its string form is remembered across launches.
 */
export type Selection =
  | { kind: "systemHosts" }
  | { kind: "profile"; id: string }
  | { kind: "hostsTag"; name: string }
  | { kind: "ports" }
  | { kind: "settings" };

export const DEFAULT_SELECTION: Selection = { kind: "systemHosts" };

export function formatSelection(selection: Selection): string {
  switch (selection.kind) {
    case "profile":
      return `profile:${selection.id}`;
    case "hostsTag":
      return `hostsTag:${selection.name}`;
    default:
      return selection.kind;
  }
}

/**
 * Reads a remembered selection; anything unreadable falls back to the hosts
 * file. That includes the `repo:` and `envFile:` panes of the removed .env
 * editor, which an older build may have remembered.
 */
export function parseSelection(raw: string | null | undefined): Selection {
  if (!raw) return DEFAULT_SELECTION;
  if (raw === "systemHosts" || raw === "ports" || raw === "settings") return { kind: raw };
  const colon = raw.indexOf(":");
  if (colon < 0) return DEFAULT_SELECTION;
  const kind = raw.slice(0, colon);
  // Split once: a block name may itself contain a colon.
  const rest = raw.slice(colon + 1);
  switch (kind) {
    case "profile":
      return rest ? { kind, id: rest } : DEFAULT_SELECTION;
    case "hostsTag":
      return rest ? { kind, name: rest } : DEFAULT_SELECTION;
    default:
      return DEFAULT_SELECTION;
  }
}

export function sameSelection(a: Selection, b: Selection): boolean {
  return formatSelection(a) === formatSelection(b);
}
