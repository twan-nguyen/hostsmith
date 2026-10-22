import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { create } from "zustand";

import {
  type EnvSnapshot,
  type HostsSnapshot,
  type PortsSnapshot,
  type Settings,
  type Toast,
  type Tools,
  envAddRepo,
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
import { type Selection, formatSelection, parseSelection, sameSelection } from "@/lib/selection";
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

export type EnvViewMode = "table" | "raw";
export type PortsFilter = "user" | "repos" | "system";

type State = {
  hosts: HostsSnapshot | null;
  env: EnvSnapshot | null;
  ports: PortsSnapshot | null;
  settings: Settings | null;
  tools: Tools | null;

  selection: Selection;
  /** Where to go back to when Settings closes. */
  lastPane: Selection;
  sidebarWidth: number;
  sidebarCollapsed: boolean;
  collapsedGroups: string[];
  envViewMode: EnvViewMode;
  portsFilter: PortsFilter;

  paletteOpen: boolean;
  /** The visible pane's search text; the palette can hand one over. Cleared when the pane changes. */
  hostsQuery: string;
  envQuery: string;
  /** Grows on every ⌘F, so the visible pane focuses its search field. */
  findRequest: number;
  /** An error from a call that has no snapshot of its own to report it. */
  notice: Toast | null;

  init: () => Promise<void>;
  select: (selection: Selection) => void;
  /** Asks for a project folder, adds it as a repo and opens it. */
  addRepo: () => Promise<void>;
  openSettings: () => void;
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
/** Both windows share one store per webview; it starts listening once. */
let started: Promise<void> | undefined;

async function start(set: (partial: Partial<State> | ((s: State) => Partial<State>)) => void, get: () => State) {
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
  // An event that arrived while the reads were out is newer than they are.
  set((s) => ({
    hosts: s.hosts ?? hosts,
    env: s.env ?? env,
    ports: s.ports ?? ports,
    settings: s.settings ?? settings,
  }));
  toolsDetect()
    .then((tools) => set({ tools }))
    .catch(() => {});
  if (getCurrentWindow().label !== "main") return;
  get().select(get().selection);
  // Another editor may have saved a dotenv file while Hosven was in the background.
  window.addEventListener("focus", () => void envRefreshFromDisk().catch(() => {}));
}

export const useStore = create<State>()((set, get) => ({
  hosts: null,
  env: null,
  ports: null,
  settings: null,
  tools: null,

  selection: parseSelection(readPref(PREFS.selection)),
  lastPane: { kind: "systemHosts" },
  sidebarWidth: clampWidth(Number(readPref(PREFS.sidebarWidth)) || SIDEBAR.initial),
  sidebarCollapsed: readPref(PREFS.sidebarCollapsed) === "true",
  collapsedGroups: (readPref(PREFS.collapsedGroups) ?? "").split(",").filter(Boolean),
  envViewMode: readPref(PREFS.envViewMode) === "raw" ? "raw" : "table",
  portsFilter: (["user", "repos", "system"] as const).find((f) => f === readPref(PREFS.portsFilter)) ?? "user",

  paletteOpen: false,
  hostsQuery: "",
  envQuery: "",
  findRequest: 0,
  notice: null,

  init: () => (started ??= start(set, get)),

  select: (selection) => {
    if (selection.kind === "settings") {
      get().openSettings();
      return;
    }
    // A search belongs to the pane it was typed in.
    if (!sameSelection(selection, get().selection)) set({ hostsQuery: "", envQuery: "" });
    set({ selection, lastPane: selection });
    writePref(PREFS.selection, formatSelection(selection));
    if (selection.kind === "repo") void get().run(envSelect(selection.id, null));
    if (selection.kind === "envFile") void get().run(envSelect(selection.repo, selection.path));
  },

  addRepo: async () => {
    const path = await open({ directory: true, title: "Chọn thư mục gốc của project" }).catch(() => null);
    if (typeof path !== "string") return;
    const before = new Set(get().env?.repos.map((r) => r.id));
    if (!(await get().run(envAddRepo(path).then(() => true)))) return;
    // Read the list now rather than wait for the event, to open the new repo.
    const env = await get().run(envSnapshot());
    if (!env) return;
    set({ env });
    const added = env.repos.find((r) => !before.has(r.id));
    if (added) get().select({ kind: "repo", id: added.id });
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
