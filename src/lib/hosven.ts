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
export type SwitchPreview = { enable: number; disable: [string, number][] };

export const hostsSnapshot = () => invoke<HostsSnapshot>("hosts_snapshot");
export const hostsReload = () => invoke<void>("hosts_reload");
export const hostsAddEntry = (input: EntryInput) => invoke<void>("hosts_add_entry", { input });
export const hostsUpdateEntry = (id: string, input: EntryInput) => invoke<void>("hosts_update_entry", { id, input });
export const hostsDuplicateEntry = (id: string) => invoke<void>("hosts_duplicate_entry", { id });
export const hostsToggleEntry = (id: string) => invoke<void>("hosts_toggle_entry", { id });
export const hostsDeleteEntry = (id: string) => invoke<void>("hosts_delete_entry", { id });
export const hostsSetTagEnabled = (name: string, enabled: boolean) =>
  invoke<void>("hosts_set_tag_enabled", { name, enabled });
export const hostsImport = (text: string) => invoke<number>("hosts_import", { text });
export const hostsUndo = () => invoke<void>("hosts_undo");
export const hostsRedo = () => invoke<void>("hosts_redo");
export const hostsDiscard = () => invoke<void>("hosts_discard");
export const hostsSetRawDraft = (text: string | null) => invoke<void>("hosts_set_raw_draft", { text });
export const hostsApply = () => invoke<Outcome>("hosts_apply");
export const hostsApplyRaw = (text: string) => invoke<Outcome>("hosts_apply_raw", { text });
export const hostsSwitchEnvironment = (target: string | null, policy: SwitchPolicy) =>
  invoke<Outcome>("hosts_switch_environment", { target, policy });
export const hostsConfirmPendingSwitch = () => invoke<Outcome>("hosts_confirm_pending_switch");
export const hostsCancelPendingSwitch = () => invoke<void>("hosts_cancel_pending_switch");
export const hostsSwitchPreview = (target: string | null) => invoke<SwitchPreview>("hosts_switch_preview", { target });
export const hostsDismissVerification = () => invoke<void>("hosts_dismiss_verification");
export const hostsExport = () => invoke<string>("hosts_export");
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

/** A repo Hosven knows; ports started from inside it count as yours. */
export type KnownRepo = { name: string; path: string };

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

export type KillOutcome = "freed" | "stillRunning" | "cancelled" | "failed";

export const portsSnapshot = () => invoke<PortsSnapshot>("ports_snapshot");
export const portsScan = (repos: KnownRepo[]) => invoke<PortsSnapshot>("ports_scan", { repos });
export const portsKill = (pid: number, force: boolean, repos: KnownRepo[]) =>
  invoke<KillOutcome>("ports_kill", { pid, force, repos });
export const portsStopContainer = (name: string, repos: KnownRepo[]) =>
  invoke<void>("ports_stop_container", { name, repos });

export const onPortsChanged = (handler: (snapshot: PortsSnapshot) => void): Promise<UnlistenFn> =>
  listen<PortsSnapshot>("ports-changed", (event) => handler(event.payload));

// MARK: - External tools and git

export type ExternalApp = { name: string; bundleId: string };
export type Tools = { terminal: ExternalApp | null; editor: ExternalApp | null; chrome: boolean };

export const toolsDetect = () => invoke<Tools>("tools_detect");
export const toolsOpenInTerminal = (path: string) => invoke<void>("tools_open_in_terminal", { path });
export const toolsOpenInEditor = (path: string) => invoke<void>("tools_open_in_editor", { path });
export const toolsRevealInFinder = (path: string) => invoke<void>("tools_reveal_in_finder", { path });
export const toolsOpenChromeDns = () => invoke<void>("tools_open_chrome_dns");

export type Exposure = "safe" | "notIgnored" | "tracked";

export const gitExposure = (repoPath: string, relativePath: string) =>
  invoke<Exposure>("git_exposure", { repoPath, relativePath });
export const gitAddToGitignore = (repoPath: string, relativePath: string) =>
  invoke<Exposure>("git_add_to_gitignore", { repoPath, relativePath });

// MARK: - Env

export type EnvEntry = {
  id: string;
  key: string;
  value: string;
  comment: string;
  enabled: boolean;
  /** Written back with the `export ` prefix the line had. */
  exported: boolean;
  isBlankOrComment: boolean;
  /** The line as it was, for blank and comment lines. */
  rawLine?: string;
};

export type EnvEntryInput = { key: string; value: string; comment: string; exported?: boolean };

/** How the file on disk differs from what Hosven last read or wrote. */
export type EnvConflict = "changed" | "missing";
export type DiscoveryState = "ok" | "repoMissing" | "envMissing";

export type EnvRepo = {
  id: string;
  name: string;
  path: string;
  missing: boolean;
  discovery: DiscoveryState;
  files: { relativePath: string; dirty: boolean; conflict: EnvConflict | null }[];
  profiles: { id: string; name: string; capturedAt: string; files: string[] }[];
};

export type EnvSnapshot = {
  repos: EnvRepo[];
  selectedRepo: string | null;
  selectedFile: string | null;
  entries: EnvEntry[];
  text: string;
  dirty: boolean;
  pendingCount: number;
  conflict: EnvConflict | null;
  loadError: string | null;
  missingKeys: string[];
  toast: Toast | null;
};

export type EnvOutcome = { kind: "written" | "nothingToDo" | "conflict" };

export const envSnapshot = () => invoke<EnvSnapshot>("env_snapshot");
export const envAddRepo = (path: string) => invoke<void>("env_add_repo", { path });
export const envRemoveRepo = (repoId: string) => invoke<void>("env_remove_repo", { repoId });
export const envRenameRepo = (repoId: string, name: string) => invoke<void>("env_rename_repo", { repoId, name });
export const envSelect = (repoId: string, relativePath: string | null) =>
  invoke<void>("env_select", { repoId, relativePath });
export const envDiscover = (repoId: string) => invoke<void>("env_discover", { repoId });
export const envAddEntry = (input: EnvEntryInput) => invoke<void>("env_add_entry", { input });
export const envUpdateEntry = (id: string, input: EnvEntryInput) => invoke<void>("env_update_entry", { id, input });
export const envDuplicateEntry = (id: string) => invoke<void>("env_duplicate_entry", { id });
export const envToggleEntry = (id: string) => invoke<void>("env_toggle_entry", { id });
export const envDeleteEntry = (id: string) => invoke<void>("env_delete_entry", { id });
export const envSelectVariant = (key: string, id: string) => invoke<void>("env_select_variant", { key, id });
export const envAddMissing = () => invoke<number>("env_add_missing");
export const envDiscard = () => invoke<void>("env_discard");
export const envSetRawDraft = (repoId: string, relativePath: string, text: string | null) =>
  invoke<void>("env_set_raw_draft", { repoId, relativePath, text });
export const envMergeRawDraft = () => invoke<void>("env_merge_raw_draft");
export const envApply = (force: boolean) => invoke<EnvOutcome>("env_apply", { force });
export const envApplyRaw = (text: string, force: boolean) => invoke<EnvOutcome>("env_apply_raw", { text, force });
export const envReloadFromDisk = () => invoke<void>("env_reload_from_disk");
export const envRefreshFromDisk = () => invoke<void>("env_refresh_from_disk");
export const envDismissConflict = () => invoke<void>("env_dismiss_conflict");
export const envProfileSave = (repoId: string, name: string) => invoke<void>("env_profile_save", { repoId, name });
export const envProfileApply = (repoId: string, profileId: string) =>
  invoke<void>("env_profile_apply", { repoId, profileId });
export const envProfileRename = (repoId: string, profileId: string, name: string) =>
  invoke<void>("env_profile_rename", { repoId, profileId, name });
export const envProfileDelete = (repoId: string, profileId: string) =>
  invoke<void>("env_profile_delete", { repoId, profileId });

export const onEnvChanged = (handler: (snapshot: EnvSnapshot) => void): Promise<UnlistenFn> =>
  listen<EnvSnapshot>("env-changed", (event) => handler(event.payload));
