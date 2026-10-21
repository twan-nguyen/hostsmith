/**
 * What the sidebar points at. One value drives the whole working pane, and
 * its string form is remembered across launches.
 */
export type Selection =
  | { kind: "systemHosts" }
  | { kind: "profile"; id: string }
  | { kind: "hostsTag"; name: string }
  | { kind: "repo"; id: string }
  | { kind: "envFile"; repo: string; path: string }
  | { kind: "ports" }
  | { kind: "settings" };

export const DEFAULT_SELECTION: Selection = { kind: "systemHosts" };

export function formatSelection(selection: Selection): string {
  switch (selection.kind) {
    case "profile":
      return `profile:${selection.id}`;
    case "hostsTag":
      return `hostsTag:${selection.name}`;
    case "repo":
      return `repo:${selection.id}`;
    case "envFile":
      return `envFile:${selection.repo}:${selection.path}`;
    default:
      return selection.kind;
  }
}

/** Reads a remembered selection; anything unreadable falls back to the hosts file. */
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
    case "repo":
      return rest ? { kind, id: rest } : DEFAULT_SELECTION;
    case "envFile": {
      const split = rest.indexOf(":");
      if (split <= 0) return DEFAULT_SELECTION;
      return { kind, repo: rest.slice(0, split), path: rest.slice(split + 1) };
    }
    default:
      return DEFAULT_SELECTION;
  }
}

export function sameSelection(a: Selection, b: Selection): boolean {
  return formatSelection(a) === formatSelection(b);
}
