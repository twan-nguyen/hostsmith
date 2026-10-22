import { fuzzyMatch } from "@/lib/fuzzy";
import type { Selection } from "@/lib/selection";

/** What running a command does; the palette component carries it out. */
export type PaletteAction =
  | { type: "select"; selection: Selection }
  | { type: "switch"; profileId: string }
  | { type: "deactivate" }
  | { type: "searchHosts"; query: string };

export type PaletteIcon = "network" | "ports" | "environment" | "off" | "search";

export type PaletteCommand = {
  /** Stable across queries, so the highlighted row survives typing. */
  id: string;
  title: string;
  subtitle?: string;
  icon: PaletteIcon;
  /** Profile colour for an environment, otherwise unset. */
  tint?: string;
  shortcut?: string;
  category: PaletteCategory;
  action: PaletteAction;
};

export type PaletteCategory = "profile" | "suggestions";

export type PaletteSection = { category: PaletteCategory; title: string; items: PaletteCommand[] };

/** What the palette needs to know about the app's state. */
export type PaletteSources = {
  environments: {
    id: string;
    name: string;
    color: string;
    shortcutNumber?: number | null;
    entryCount: number;
    active: boolean;
  }[];
  /** Whether any environment, or several, has entries on. */
  anyActive: boolean;
};

const SECTION_TITLES: Record<PaletteCategory, string> = {
  profile: "Profile",
  suggestions: "Gợi ý",
};
const ORDER: PaletteCategory[] = ["profile", "suggestions"];
/** Enough to spot the right one without a long hosts file drowning the list. */
export const MAX_PER_SECTION = 8;

function universe(sources: PaletteSources): PaletteCommand[] {
  const commands: PaletteCommand[] = [
    {
      id: "open-systemHosts",
      title: "Mở Hosts hệ thống",
      subtitle: "/etc/hosts",
      icon: "network",
      category: "suggestions",
      action: { type: "select", selection: { kind: "systemHosts" } },
    },
    {
      id: "open-ports",
      title: "Mở Ports",
      subtitle: "Cổng đang bị chiếm",
      icon: "ports",
      category: "suggestions",
      action: { type: "select", selection: { kind: "ports" } },
    },
  ];

  for (const env of sources.environments) {
    if (env.entryCount === 0) continue;
    const n = env.shortcutNumber;
    commands.push({
      id: `switch-profile-${env.id}`,
      title: `Chuyển sang ${env.name}`,
      subtitle: env.active ? "Đang hoạt động" : undefined,
      icon: "environment",
      tint: env.color,
      shortcut: n && n >= 1 && n <= 9 ? `⌘${n}` : undefined,
      category: "profile",
      action: { type: "switch", profileId: env.id },
    });
  }
  if (sources.anyActive) {
    commands.push({
      id: "clear-profile",
      title: "Tắt môi trường đang bật",
      subtitle: "Ghi vào /etc/hosts",
      icon: "off",
      shortcut: "⌘0",
      category: "profile",
      action: { type: "deactivate" },
    });
  }
  return commands;
}

function searchJump(query: string): PaletteCommand {
  return {
    id: "search-hosts",
    title: `Tìm "${query}" trong Hosts`,
    subtitle: "Lọc theo hostname / IP",
    icon: "search",
    category: "suggestions",
    action: { type: "searchHosts", query },
  };
}

/** The palette's sections for `query`, best match first in each. */
export function paletteSections(query: string, sources: PaletteSources): PaletteSection[] {
  const trimmed = query.trim();
  let scored: [PaletteCommand, number][];
  if (trimmed === "") {
    scored = universe(sources).map((command) => [command, 0]);
  } else {
    scored = universe(sources).flatMap((command): [PaletteCommand, number][] => {
      const title = fuzzyMatch(trimmed, command.title) ?? -Infinity;
      const subtitle = command.subtitle ? (fuzzyMatch(trimmed, command.subtitle) ?? -Infinity) : -Infinity;
      const best = Math.max(title, subtitle * 0.6);
      return best === -Infinity ? [] : [[command, best]];
    });
    // Score 0, so a command that matches by name still ranks above it.
    scored.push([searchJump(trimmed), 0]);
  }

  return ORDER.flatMap((category) => {
    const items = scored
      .filter(([command]) => command.category === category)
      .sort((a, b) => b[1] - a[1])
      .slice(0, MAX_PER_SECTION)
      .map(([command]) => command);
    return items.length ? [{ category, title: SECTION_TITLES[category], items }] : [];
  });
}
