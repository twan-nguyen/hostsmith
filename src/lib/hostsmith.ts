import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// MARK: - Shared

export type ToastKind = "success" | "error" | "info";
/** `id` grows with every toast, so a repeated snapshot does not show it twice. */
export type Toast = { id: number; message: string; kind: ToastKind };

/** How an action that may write a file ended. */
export type Outcome =
  | { kind: "written" }
  | { kind: "cancelled" }
  | { kind: "failed"; value: string }
  | { kind: "needsConfirmation"; value: number }
  | { kind: "refused"; value: number }
  | { kind: "nothingToDo" };

// MARK: - Hosts

export type HostEntry = {
  id: string;
  ip: string;
  hostname: string;
  aliases: string[];
  comment: string;
  enabled: boolean;
  tag: string | null;
};

export type ProfileColor = "purple" | "green" | "amber" | "blue" | "red";

export type Profile = {
  id: string;
  name: string;
  color: ProfileColor;
  shortcutNumber?: number;
  isExclusive: boolean;
};

export type ActiveState = { kind: "none" } | { kind: "active"; id: string } | { kind: "conflict"; ids: string[] };

export type Check = { hostname: string; expectedIp: string; resolvedIps: string[]; matches: boolean };

export type HostsSnapshot = {
  text: string;
  entries: HostEntry[];
  tags: string[];
  profiles: Profile[];
  active: ActiveState;
  dirty: boolean;
  pendingCount: number;
  canUndo: boolean;
  canRedo: boolean;
  applying: boolean;
  externalChange: boolean;
  loadError: string | null;
  verification: { results: Check[] } | null;
  pendingSwitch: { target: string | null; pendingCount: number } | null;
  toast: Toast | null;
};

export type EntryInput = {
  ip: string;
  hostname: string;
  /** Leave out to keep the aliases the entry has. */
  aliases?: string[];
  comment: string;
  tag: string | null;
};

export type SwitchPolicy = "ask" | "refuse" | "include";

export const hostsSnapshot = () => invoke<HostsSnapshot>("hosts_snapshot");
export const hostsReload = () => invoke<void>("hosts_reload");
export const hostsAddEntry = (input: EntryInput) => invoke<void>("hosts_add_entry", { input });
export const hostsUpdateEntry = (id: string, input: EntryInput) => invoke<void>("hosts_update_entry", { id, input });
export const hostsDuplicateEntry = (id: string) => invoke<void>("hosts_duplicate_entry", { id });
export const hostsToggleEntry = (id: string) => invoke<void>("hosts_toggle_entry", { id });
export const hostsDeleteEntry = (id: string) => invoke<void>("hosts_delete_entry", { id });
export const hostsSetTagEnabled = (name: string, enabled: boolean) =>
  invoke<void>("hosts_set_tag_enabled", { name, enabled });
export const hostsUndo = () => invoke<void>("hosts_undo");
export const hostsRedo = () => invoke<void>("hosts_redo");
export const hostsDiscard = () => invoke<void>("hosts_discard");
/** `base` is the model text the typing started from; a draft over older text is dropped. */
export const hostsSetRawDraft = (text: string | null, base: string) =>
  invoke<void>("hosts_set_raw_draft", { text, base });
export const hostsApply = () => invoke<Outcome>("hosts_apply");
export const hostsApplyRaw = (text: string, base: string) => invoke<Outcome>("hosts_apply_raw", { text, base });
export const hostsSwitchEnvironment = (target: string | null, policy: SwitchPolicy) =>
  invoke<Outcome>("hosts_switch_environment", { target, policy });
export const hostsConfirmPendingSwitch = () => invoke<Outcome>("hosts_confirm_pending_switch");
export const hostsCancelPendingSwitch = () => invoke<void>("hosts_cancel_pending_switch");
export const hostsDismissVerification = () => invoke<void>("hosts_dismiss_verification");
/** Copies /etc/hosts beside itself as root; the outcome arrives as a toast. */
export const hostsBackup = () => invoke<void>("hosts_backup");

export const profilesAdd = (name: string, color: ProfileColor, isExclusive: boolean) =>
  invoke<Profile>("profiles_add", { name, color, isExclusive });
export const profilesRename = (id: string, name: string) => invoke<void>("profiles_rename", { id, name });
export const profilesRemove = (id: string) => invoke<void>("profiles_remove", { id });
export const profilesSetExclusive = (id: string, isExclusive: boolean) =>
  invoke<void>("profiles_set_exclusive", { id, isExclusive });
export const profilesSetColor = (id: string, color: ProfileColor) => invoke<void>("profiles_set_color", { id, color });

export const onHostsChanged = (handler: (snapshot: HostsSnapshot) => void): Promise<UnlistenFn> =>
  listen<HostsSnapshot>("hosts-changed", (event) => handler(event.payload));

// MARK: - Settings

export type AppearanceMode = "system" | "light" | "dark";

export type Settings = {
  appearanceMode: AppearanceMode;
  editorFontSize: number;
  showMenuBarExtra: boolean;
  transparency: number;
};

export const settingsGet = () => invoke<Settings>("settings_get");
export const settingsUpdate = (patch: Partial<Settings>) => invoke<Settings>("settings_update", { patch });
export const settingsPreviewTransparency = (percent: number) =>
  invoke<void>("settings_preview_transparency", { percent });

export const onSettingsChanged = (handler: (settings: Settings) => void): Promise<UnlistenFn> =>
  listen<Settings>("settings-changed", (event) => handler(event.payload));

// MARK: - Ports

/** `yours` carries the project: the nearest folder holding `.git` above the process's working directory. */
export type PortOwner = { kind: "yours"; name: string } | { kind: "container"; name: string | null } | { kind: "unknown" };

export type ListeningPort = {
  id: string;
  port: number;
  process: string;
  pid: number;
  user: string;
  cwd: string | null;
  owner: PortOwner;
  isSystem: boolean;
  needsAttention: boolean;
};

export type PortsSnapshot = {
  ports: ListeningPort[];
  /** Milliseconds since the Unix epoch. */
  scannedAt: number | null;
  scanning: boolean;
  toast: Toast | null;
};

export type KillOutcome = "freed" | "stillRunning" | "failed";

export const portsSnapshot = () => invoke<PortsSnapshot>("ports_snapshot");
export const portsScan = () => invoke<PortsSnapshot>("ports_scan");
export const portsKill = (pid: number, force: boolean) => invoke<KillOutcome>("ports_kill", { pid, force });
export const portsStopContainer = (name: string) => invoke<void>("ports_stop_container", { name });

export const onPortsChanged = (handler: (snapshot: PortsSnapshot) => void): Promise<UnlistenFn> =>
  listen<PortsSnapshot>("ports-changed", (event) => handler(event.payload));

// MARK: - External tools

export type ExternalApp = { name: string; bundleId: string };
export type Tools = { terminal: ExternalApp | null; editor: ExternalApp | null; chrome: boolean };

export const toolsDetect = () => invoke<Tools>("tools_detect");
export const toolsOpenInTerminal = (path: string) => invoke<void>("tools_open_in_terminal", { path });
export const toolsOpenInEditor = (path: string) => invoke<void>("tools_open_in_editor", { path });
export const toolsRevealInFinder = (path: string) => invoke<void>("tools_reveal_in_finder", { path });
export const toolsOpenChromeDns = () => invoke<void>("tools_open_chrome_dns");

/** Tells Rust the main window listens, so links that opened the app run now. */
export const windowReady = () => invoke<void>("window_ready");
