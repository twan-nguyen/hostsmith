import { create } from "zustand";

import {
  type EnvSnapshot,
  type HostsSnapshot,
  type KnownRepo,
  type PortsSnapshot,
  type Settings,
  type Toast,
  type Tools,
  envRefreshFromDisk,
  envSelect,
  envSnapshot,
  hostsSnapshot,
  onEnvChanged,
  onHostsChanged,
  onPortsChanged,
  onSettingsChanged,
  portsSnapshot,
  settingsGet,
  toolsDetect,
} from "@/lib/hosven";
import { type Selection, formatSelection, parseSelection } from "@/lib/selection";
import { errorMessage } from "@/lib/utils";

/** Layout and view preferences that live in the webview, per machine. */
const PREFS = {
  selection: "hosven.sidebarSelection",
  sidebarWidth: "hosven.sidebarWidth",
  sidebarCollapsed: "hosven.sidebarCollapsed",
  collapsedGroups: "hosven.sidebarCollapsedGroups",
  envViewMode: "hosven.envViewMode",
  portsFilter: "hosven.portsFilter",
} as const;

// Storage can be unavailable or full; every preference has a default.
export function readPref(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

export function writePref(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Keeps working with the in-memory value.
  }
}

export const SIDEBAR = { min: 200, max: 320, collapseBelow: 140, initial: 248 } as const;

export type EnvViewMode = "table" | "raw";
export type PortsFilter = "all" | "repos" | "system";
export type SettingsPage = "general" | "profiles";

type State = {
  hosts: HostsSnapshot | null;
  env: EnvSnapshot | null;
  ports: PortsSnapshot | null;
  settings: Settings | null;
  tools: Tools | null;

  selection: Selection;
  /** Where to go back to when Settings closes. */
  lastPane: Selection;
  settingsPage: SettingsPage;
  sidebarWidth: number;
  sidebarCollapsed: boolean;
  collapsedGroups: string[];
  envViewMode: EnvViewMode;
  portsFilter: PortsFilter;

  paletteOpen: boolean;
  /** Search text handed to a pane, by the palette or ⌘F. */
  hostsQuery: string;
  envQuery: string;
  /** Grows on every ⌘F, so the visible pane focuses its search field. */
  findRequest: number;
  /** An error from a call that has no snapshot of its own to report it. */
  notice: Toast | null;

  init: () => Promise<void>;
  select: (selection: Selection) => void;
  openSettings: (page?: SettingsPage) => void;
  setSettingsPage: (page: SettingsPage) => void;
  closeSettings: () => void;
  setSidebarWidth: (width: number) => void;
  toggleSidebar: () => void;
  toggleGroup: (group: string) => void;
  setEnvViewMode: (mode: EnvViewMode) => void;
  setPortsFilter: (filter: PortsFilter) => void;
  setPaletteOpen: (open: boolean) => void;
  setHostsQuery: (query: string) => void;
  setEnvQuery: (query: string) => void;
  requestFind: () => void;
  /** Runs an IPC call, reporting a failure as a notice instead of throwing. */
  run: <T>(action: Promise<T>) => Promise<T | undefined>;
  dismissNotice: () => void;
};

let noticeSeq = 0;

export const useStore = create<State>()((set, get) => ({
  hosts: null,
  env: null,
  ports: null,
  settings: null,
  tools: null,

  selection: parseSelection(readPref(PREFS.selection)),
  lastPane: { kind: "systemHosts" },
  settingsPage: "general",
  sidebarWidth: clampWidth(Number(readPref(PREFS.sidebarWidth)) || SIDEBAR.initial),
  sidebarCollapsed: readPref(PREFS.sidebarCollapsed) === "true",
  collapsedGroups: (readPref(PREFS.collapsedGroups) ?? "").split(",").filter(Boolean),
  envViewMode: readPref(PREFS.envViewMode) === "raw" ? "raw" : "table",
  portsFilter: (["all", "repos", "system"] as const).find((f) => f === readPref(PREFS.portsFilter)) ?? "all",

  paletteOpen: false,
  hostsQuery: "",
  envQuery: "",
  findRequest: 0,
  notice: null,

  init: async () => {
    // Listen first, so no change between the reads and the listeners is lost.
    await Promise.all([
      onHostsChanged((hosts) => set({ hosts })),
      onEnvChanged((env) => set({ env })),
      onPortsChanged((ports) => set({ ports })),
      onSettingsChanged((settings) => set({ settings })),
    ]);
    const [hosts, env, ports, settings] = await Promise.all([
      hostsSnapshot(),
      envSnapshot(),
      portsSnapshot(),
      settingsGet(),
    ]);
    set({ hosts, env, ports, settings });
    get().select(get().selection);
    toolsDetect()
      .then((tools) => set({ tools }))
      .catch(() => {});
    // Another editor may have saved a dotenv file while Hosven was in the background.
    window.addEventListener("focus", () => void envRefreshFromDisk().catch(() => {}));
  },

  select: (selection) => {
    if (selection.kind === "settings") {
      get().openSettings();
      return;
    }
    set({ selection, lastPane: selection });
    writePref(PREFS.selection, formatSelection(selection));
    if (selection.kind === "repo") void get().run(envSelect(selection.id, null));
    if (selection.kind === "envFile") void get().run(envSelect(selection.repo, selection.path));
  },

  openSettings: (page) => {
    const { selection, settingsPage } = get();
    set({ settingsPage: page ?? settingsPage });
    if (selection.kind !== "settings") set({ lastPane: selection, selection: { kind: "settings" } });
  },

  setSettingsPage: (settingsPage) => set({ settingsPage }),

  closeSettings: () => {
    const { selection, lastPane } = get();
    if (selection.kind === "settings") set({ selection: lastPane });
  },

  setSidebarWidth: (width) => {
    if (width < SIDEBAR.collapseBelow) {
      set({ sidebarCollapsed: true });
      writePref(PREFS.sidebarCollapsed, "true");
      return;
    }
    const sidebarWidth = clampWidth(width);
    set({ sidebarWidth, sidebarCollapsed: false });
    writePref(PREFS.sidebarWidth, String(sidebarWidth));
    writePref(PREFS.sidebarCollapsed, "false");
  },

  toggleSidebar: () => {
    const sidebarCollapsed = !get().sidebarCollapsed;
    set({ sidebarCollapsed });
    writePref(PREFS.sidebarCollapsed, String(sidebarCollapsed));
  },

  toggleGroup: (group) => {
    const groups = get().collapsedGroups;
    const collapsedGroups = groups.includes(group) ? groups.filter((g) => g !== group) : [...groups, group];
    set({ collapsedGroups });
    writePref(PREFS.collapsedGroups, collapsedGroups.join(","));
  },

  setEnvViewMode: (envViewMode) => {
    set({ envViewMode });
    writePref(PREFS.envViewMode, envViewMode);
  },

  setPortsFilter: (portsFilter) => {
    set({ portsFilter });
    writePref(PREFS.portsFilter, portsFilter);
  },

  setPaletteOpen: (paletteOpen) => set({ paletteOpen }),
  setHostsQuery: (hostsQuery) => set({ hostsQuery }),
  setEnvQuery: (envQuery) => set({ envQuery }),
  requestFind: () => set({ findRequest: get().findRequest + 1 }),

  run: async (action) => {
    try {
      return await action;
    } catch (error) {
      set({ notice: { id: ++noticeSeq, message: errorMessage(error), kind: "error" } });
      return undefined;
    }
  },

  dismissNotice: () => set({ notice: null }),
}));

function clampWidth(width: number) {
  return Math.min(SIDEBAR.max, Math.max(SIDEBAR.min, width));
}

/** The repos ports are attributed to: every repo Hosven knows. */
export function knownRepos(env: EnvSnapshot | null): KnownRepo[] {
  return env?.repos.map((r) => ({ name: r.name, path: r.path })) ?? [];
}

/** Environment and block profiles, with how many entries each block holds. */
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
