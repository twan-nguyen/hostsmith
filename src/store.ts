import { getCurrentWindow } from "@tauri-apps/api/window";
import { create } from "zustand";

import {
  type HostsSnapshot,
  type PortsSnapshot,
  type Settings,
  type Toast,
  type Tools,
  hostsSnapshot,
  onHostsChanged,
  onPortsChanged,
  onSettingsChanged,
  portsSnapshot,
  settingsGet,
  toolsDetect,
} from "@/lib/hostsmith";
import { type Selection, formatSelection, parseSelection, sameSelection } from "@/lib/selection";
import { errorMessage } from "@/lib/utils";

/** Layout and view preferences that live in the webview, per machine. */
const PREFS = {
  selection: "hostsmith.sidebarSelection",
  sidebarWidth: "hostsmith.sidebarWidth",
  sidebarCollapsed: "hostsmith.sidebarCollapsed",
  collapsedGroups: "hostsmith.sidebarCollapsedGroups",
  // Renamed when "Projects" became the default, so an older choice does not stick.
  portsFilter: "hostsmith.portsView",
} as const;

// Storage can be unavailable or full; every preference has a default.
function readPref(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function writePref(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Keeps working with the in-memory value.
  }
}

export const SIDEBAR = { min: 200, max: 320, collapseBelow: 140, initial: 248 } as const;

export type PortsFilter = "user" | "projects" | "system";

type State = {
  hosts: HostsSnapshot | null;
  ports: PortsSnapshot | null;
  settings: Settings | null;
  tools: Tools | null;

  selection: Selection;
  /** Where to go back to when Settings closes. */
  lastPane: Selection;
  sidebarWidth: number;
  sidebarCollapsed: boolean;
  collapsedGroups: string[];
  portsFilter: PortsFilter;

  paletteOpen: boolean;
  /** The visible pane's search text; the palette can hand one over. Cleared when the pane changes. */
  hostsQuery: string;
  /** Grows on every ⌘F, so the visible pane focuses its search field. */
  findRequest: number;
  /** An error from a call that has no snapshot of its own to report it. */
  notice: Toast | null;

  init: () => Promise<void>;
  select: (selection: Selection) => void;
  openSettings: () => void;
  closeSettings: () => void;
  setSidebarWidth: (width: number) => void;
  toggleSidebar: () => void;
  toggleGroup: (group: string) => void;
  setPortsFilter: (filter: PortsFilter) => void;
  setPaletteOpen: (open: boolean) => void;
  setHostsQuery: (query: string) => void;
  requestFind: () => void;
  /** Runs an IPC call, reporting a failure as a notice instead of throwing. */
  run: <T>(action: Promise<T>) => Promise<T | undefined>;
  dismissNotice: () => void;
};

let noticeSeq = 0;
/** Both windows share one store per webview; it starts listening once. */
let started: Promise<void> | undefined;

async function start(set: (partial: Partial<State> | ((s: State) => Partial<State>)) => void, get: () => State) {
  // Listen first, so no change between the reads and the listeners is lost.
  await Promise.all([
    onHostsChanged((hosts) => set({ hosts })),
    onPortsChanged((ports) => set({ ports })),
    onSettingsChanged((settings) => set({ settings })),
  ]);
  const [hosts, ports, settings] = await Promise.all([hostsSnapshot(), portsSnapshot(), settingsGet()]);
  // An event that arrived while the reads were out is newer than they are.
  set((s) => ({
    hosts: s.hosts ?? hosts,
    ports: s.ports ?? ports,
    settings: s.settings ?? settings,
  }));
  toolsDetect()
    .then((tools) => set({ tools }))
    .catch(() => {});
  if (getCurrentWindow().label !== "main") return;
  get().select(get().selection);
}

export const useStore = create<State>()((set, get) => ({
  hosts: null,
  ports: null,
  settings: null,
  tools: null,

  selection: parseSelection(readPref(PREFS.selection)),
  lastPane: { kind: "systemHosts" },
  sidebarWidth: clampWidth(Number(readPref(PREFS.sidebarWidth)) || SIDEBAR.initial),
  sidebarCollapsed: readPref(PREFS.sidebarCollapsed) === "true",
  collapsedGroups: (readPref(PREFS.collapsedGroups) ?? "").split(",").filter(Boolean),
  portsFilter: (["user", "projects", "system"] as const).find((f) => f === readPref(PREFS.portsFilter)) ?? "projects",

  paletteOpen: false,
  hostsQuery: "",
  findRequest: 0,
  notice: null,

  init: () => (started ??= start(set, get)),

  select: (selection) => {
    if (selection.kind === "settings") {
      get().openSettings();
      return;
    }
    // A search belongs to the pane it was typed in.
    if (!sameSelection(selection, get().selection)) set({ hostsQuery: "" });
    set({ selection, lastPane: selection });
    writePref(PREFS.selection, formatSelection(selection));
  },

  openSettings: () => {
    const { selection } = get();
    if (selection.kind !== "settings") set({ lastPane: selection, selection: { kind: "settings" } });
  },

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

  setPortsFilter: (portsFilter) => {
    set({ portsFilter });
    writePref(PREFS.portsFilter, portsFilter);
  },

  setPaletteOpen: (paletteOpen) => set({ paletteOpen }),
  setHostsQuery: (hostsQuery) => set({ hostsQuery }),
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
