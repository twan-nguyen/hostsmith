import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";

import type { HostEntry, HostsSnapshot, PortsSnapshot, Profile, Settings, Tools } from "@/lib/hostsmith";

/**
 * Sample data for opening the UI in a plain browser (`pnpm dev`, then
 * http://localhost:1420), where there is no Rust side. Only the development
 * build loads this; it never writes anything.
 */

const profiles: Profile[] = [
  { id: "p-local", name: "Local", color: "blue", shortcutNumber: 1, isExclusive: true },
  { id: "p-staging", name: "Staging", color: "amber", shortcutNumber: 2, isExclusive: true },
  { id: "p-prod", name: "Production", color: "red", shortcutNumber: 3, isExclusive: true },
  { id: "p-mocks", name: "Mocks", color: "purple", isExclusive: false },
  { id: "p-ads", name: "Chặn quảng cáo", color: "green", isExclusive: false },
];

let seq = 0;
const entry = (ip: string, hostname: string, tag: string | null, enabled = true, comment = ""): HostEntry => ({
  id: `e-${++seq}`,
  ip,
  hostname,
  aliases: [],
  comment,
  enabled,
  tag,
});

const entries: HostEntry[] = [
  entry("127.0.0.1", "localhost", null),
  entry("255.255.255.255", "broadcasthost", null),
  entry("::1", "localhost", null),
  entry("127.0.0.1", "api.shop.test", "Local", false, "API chạy bằng docker compose"),
  entry("127.0.0.1", "web.shop.test", "Local", false),
  entry("127.0.0.1", "admin.shop.test", "Local", false),
  entry("10.20.0.14", "api.shop.example.com", "Staging", true, "staging-api-01"),
  entry("10.20.0.15", "web.shop.example.com", "Staging", true),
  entry("10.20.0.16", "admin.shop.example.com", "Staging", true),
  entry("10.20.0.30", "cdn.shop.example.com", "Staging", false, "tạm tắt"),
  entry("52.74.10.8", "api.shop.example.com", "Production", false),
  entry("52.74.10.9", "web.shop.example.com", "Production", false),
  entry("127.0.0.1", "payments.mock", "Mocks", true),
  entry("127.0.0.1", "sms.mock", "Mocks", true),
  entry("0.0.0.0", "ads.doubleclick.net", "Chặn quảng cáo", false),
  entry("0.0.0.0", "pagead2.googlesyndication.com", "Chặn quảng cáo", false),
];

function text(): string {
  const lines: string[] = [];
  const tags = [...new Set(entries.map((e) => e.tag).filter((t): t is string => t !== null))];
  const line = (e: HostEntry) => `${e.enabled ? "" : "# "}${e.ip}\t${e.hostname}${e.comment ? ` # ${e.comment}` : ""}`;
  for (const e of entries.filter((e) => e.tag === null)) lines.push(line(e));
  for (const tag of tags) {
    lines.push("", `## [tag:${tag}]`);
    for (const e of entries.filter((e) => e.tag === tag)) lines.push(line(e));
    lines.push(`## [/tag:${tag}]`);
  }
  return lines.join("\n") + "\n";
}

/** `?alerts` opens with the standing problems the toaster keeps on screen. */
const alerts = new URLSearchParams(location.search).has("alerts");

let toastSeq = 0;
let toast: HostsSnapshot["toast"] = null;

function hosts(): HostsSnapshot {
  const on = profiles.filter((p) => p.isExclusive && entries.some((e) => e.tag === p.name && e.enabled));
  return {
    text: text(),
    entries,
    tags: profiles.map((p) => p.name),
    profiles,
    active: on.length === 0 ? { kind: "none" } : on.length === 1 ? { kind: "active", id: on[0].id } : { kind: "conflict", ids: on.map((p) => p.id) },
    dirty: false,
    pendingCount: 0,
    canUndo: false,
    canRedo: false,
    applying: false,
    externalChange: alerts,
    loadError: null,
    verification: alerts
      ? {
          results: [
            { hostname: "api.shop.example.com", expectedIp: "10.20.0.14", resolvedIps: ["52.74.10.8"], matches: false },
            { hostname: "web.shop.example.com", expectedIp: "10.20.0.15", resolvedIps: ["10.20.0.15"], matches: true },
          ],
        }
      : null,
    pendingSwitch: null,
    toast,
  };
}

const ports: PortsSnapshot = {
  scannedAt: Date.now(),
  scanning: false,
  toast: null,
  ports: [
    { id: "3000-1", port: 3000, process: "node", pid: 41210, user: "me", cwd: "/Users/me/code/shop-web", owner: { kind: "yours", name: "shop-web" }, isSystem: false, needsAttention: false },
    { id: "5432-1", port: 5432, process: "com.docker.backend", pid: 811, user: "me", cwd: null, owner: { kind: "container", name: "shop-postgres" }, isSystem: false, needsAttention: false },
    { id: "8080-1", port: 8080, process: "java", pid: 52011, user: "me", cwd: "/Users/me/code/shop-api", owner: { kind: "yours", name: "shop-api" }, isSystem: false, needsAttention: false },
    { id: "9229-1", port: 9229, process: "node", pid: 61002, user: "me", cwd: "/private/tmp", owner: { kind: "unknown" }, isSystem: false, needsAttention: true },
    { id: "5000-1", port: 5000, process: "ControlCenter", pid: 512, user: "me", cwd: "/", owner: { kind: "unknown" }, isSystem: true, needsAttention: false },
  ],
};

let settings: Settings = { appearanceMode: new URLSearchParams(location.search).has("light") ? "light" : "dark", editorFontSize: 13, showMenuBarExtra: true, transparency: 0 };
const tools: Tools = { terminal: { name: "Ghostty", bundleId: "x" }, editor: { name: "Zed", bundleId: "y" }, chrome: true };

function changed() {
  void emit("hosts-changed", hosts());
}

function say(message: string) {
  toast = { id: ++toastSeq, message, kind: "success" };
}

export function installMockIpc() {
  mockWindows("main");
  mockIPC(
    (cmd, args) => {
      const a = (args ?? {}) as Record<string, unknown>;
      switch (cmd) {
        case "hosts_snapshot":
          return hosts();
        case "ports_snapshot":
        case "ports_scan":
          return ports;
        case "settings_get":
          return settings;
        case "tools_detect":
          return tools;
        case "settings_update":
          settings = { ...settings, ...(a.patch as Partial<Settings>) };
          void emit("settings-changed", settings);
          return settings;
        case "hosts_toggle_entry": {
          const e = entries.find((e) => e.id === a.id);
          if (e) e.enabled = !e.enabled;
          changed();
          return null;
        }
        case "hosts_set_tag_enabled": {
          for (const e of entries.filter((e) => e.tag === a.name)) e.enabled = Boolean(a.enabled);
          say(`Đã ${a.enabled ? "bật" : "tắt"} block ${String(a.name)}`);
          changed();
          return null;
        }
        case "hosts_switch_environment": {
          const target = profiles.find((p) => p.id === a.target);
          for (const e of entries) {
            const p = profiles.find((p) => p.name === e.tag);
            if (p?.isExclusive) e.enabled = p.id === target?.id;
          }
          say(target ? `Đã chuyển sang ${target.name}` : "Đã tắt môi trường");
          changed();
          return { kind: "written" };
        }
        default:
          return null;
      }
    },
    { shouldMockEvents: true },
  );
}
