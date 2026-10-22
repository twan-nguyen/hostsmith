import { LogicalSize } from "@tauri-apps/api/dpi";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ChevronDown, Layers } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { Banner } from "@/components/banner";
import { Switch } from "@/components/ui/switch";
import { type Outcome, hostsSwitchEnvironment, portsScan } from "@/lib/hostsmith";
import {
  MAX_PORTS,
  MAX_TOGGLES,
  activeTitle,
  blockRows,
  environmentRows,
  outcomeMessage,
  panelHide,
  panelOpenMain,
  panelQuit,
  panelToggleBlock,
  portTitle,
  portTrailing,
  userPorts,
} from "@/lib/menubar";
import { PROFILE_COLORS } from "@/lib/profile-color";
import { cn, errorMessage } from "@/lib/utils";
import { useStore } from "@/store";

const PANEL_WIDTH = 340;
const darkScheme = window.matchMedia("(prefers-color-scheme: dark)");

/**
 * The panel that drops from the menu bar icon: the active environment, up to
 * eight block switches, up to three ports and a footer. It never scrolls;
 * the window takes the height of its content.
 */
export function MenubarPanel() {
  const init = useStore((s) => s.init);
  const hosts = useStore((s) => s.hosts);
  const ports = useStore((s) => s.ports);
  const mode = useStore((s) => s.settings?.appearanceMode ?? "system");
  const [switcherOpen, setSwitcherOpen] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  const root = useRef<HTMLDivElement>(null);

  useEffect(() => {
    void init();
  }, [init]);

  // Each time the panel opens it scans the ports; when it closes it forgets
  // the switcher and any failure, as a freshly opened menu would.
  useEffect(() => {
    const unlisten = getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused) {
        void portsScan().catch(() => {});
      } else {
        setSwitcherOpen(false);
        setFailure(null);
      }
    });
    return () => void unlisten.then((f) => f());
  }, []);

  useEffect(() => {
    const apply = () =>
      document.documentElement.classList.toggle("dark", mode === "dark" || (mode === "system" && darkScheme.matches));
    apply();
    darkScheme.addEventListener("change", apply);
    return () => darkScheme.removeEventListener("change", apply);
  }, [mode]);

  // The window follows the content's height.
  useEffect(() => {
    const el = root.current;
    if (!el) return;
    const observer = new ResizeObserver(() => {
      const height = Math.ceil(el.getBoundingClientRect().height);
      void getCurrentWindow()
        .setSize(new LogicalSize(PANEL_WIDTH, height))
        .catch(() => {});
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        void panelHide();
      } else if (event.metaKey && event.key.toLowerCase() === "o") {
        event.preventDefault();
        void panelOpenMain();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  /** Runs a write from the panel, saying in the panel why it did not land. */
  const write = async (action: Promise<Outcome>) => {
    setFailure(null);
    try {
      const outcome = await action;
      setFailure(outcomeMessage(outcome));
      return outcome;
    } catch (error) {
      setFailure(errorMessage(error));
      return undefined;
    }
  };

  const switchTo = async (target: string | null) => {
    const outcome = await write(hostsSwitchEnvironment(target, "refuse"));
    if (outcome?.kind === "written") setSwitcherOpen(false);
  };

  const { title, profile: activeProfile } = activeTitle(hosts);
  const environments = environmentRows(hosts);
  const blocks = blockRows(hosts);
  const shownBlocks = blocks.slice(0, MAX_TOGGLES);
  const portList = userPorts(ports);
  const shownPorts = portList.slice(0, MAX_PORTS);
  const busy = hosts?.applying ?? false;

  return (
    <div ref={root} className="p-1">
      <div className="flex flex-col rounded-xl border border-border bg-popover p-1.5 text-popover-foreground shadow-[0_2px_8px_oklch(0_0_0/18%)]">
        <button
          onClick={() => setSwitcherOpen((open) => !open)}
          aria-expanded={switcherOpen}
          aria-label={activeProfile ? `Môi trường đang bật: ${activeProfile.name}` : "Không có môi trường đang bật"}
          className="flex items-center gap-3 rounded-lg bg-muted/70 px-3 py-2.5 text-left transition-colors hover:bg-muted"
        >
          <Layers className="size-4 shrink-0 text-muted-foreground" />
          <span className="min-w-0 flex-1 truncate text-[15px] font-medium">{title}</span>
          {activeProfile ? (
            <span
              className={cn(
                "shrink-0 rounded-full px-2 py-0.5 text-[11px] font-medium",
                PROFILE_COLORS[activeProfile.color].chip,
              )}
            >
              {PROFILE_COLORS[activeProfile.color].label}
            </span>
          ) : (
            <ChevronDown
              className={cn("size-3.5 shrink-0 text-muted-foreground transition-transform", switcherOpen && "rotate-180")}
            />
          )}
        </button>

        {switcherOpen && (
          <div className="flex animate-fade-in flex-col pt-1">
            {environments.map((row) => (
              <Row
                key={row.profile.id}
                title={row.profile.name}
                subtitle={`${row.entryCount} entry`}
                trailing={row.active ? "Đang bật" : undefined}
                disabled={busy}
                onClick={() => void switchTo(row.profile.id)}
              />
            ))}
            {hosts && hosts.active.kind !== "none" && (
              <Row title="Tắt môi trường" disabled={busy} onClick={() => void switchTo(null)} />
            )}
          </div>
        )}

        <Separator />

        {failure && (
          <Banner tone="danger" title={failure} onDismiss={() => setFailure(null)} className="mx-1 mb-1.5 px-3 py-2" />
        )}

        <GroupLabel>Bật nhanh</GroupLabel>
        {shownBlocks.length === 0 && <Empty>Chưa có block nào trong /etc/hosts</Empty>}
        {shownBlocks.map((row) => (
          <div key={row.profile.id} className="flex min-h-9 items-center gap-3 px-3 py-1">
            <div className="flex min-w-0 flex-1 flex-col">
              <span className="truncate">{row.profile.name}</span>
              <span className="truncate font-mono text-[11px] text-muted-foreground">{row.mapping}</span>
            </div>
            <Switch
              size="sm"
              checked={row.on}
              disabled={busy}
              aria-label={`${row.profile.name}, ${row.on ? "bật" : "tắt"}`}
              onCheckedChange={(checked) => void write(panelToggleBlock(row.profile.name, checked))}
            />
          </div>
        ))}
        {blocks.length > shownBlocks.length && (
          <Row title="Xem tất cả trong HostSmith" accent onClick={() => void panelOpenMain()} />
        )}

        <Separator />

        <GroupLabel>Cổng đang dùng · {portList.length}</GroupLabel>
        {shownPorts.length === 0 && (
          <Empty>{ports?.scanning ? "Đang quét" : "Không có cổng nào đang lắng nghe"}</Empty>
        )}
        {shownPorts.map((port) => (
          <Row
            key={port.id}
            title={portTitle(port)}
            trailing={portTrailing(port)}
            danger={port.needsAttention}
            onClick={() => void panelOpenMain("ports")}
          />
        ))}
        {portList.length > shownPorts.length && (
          <Row title="Xem tất cả trong HostSmith" accent onClick={() => void panelOpenMain("ports")} />
        )}

        <Separator />

        <Row title="Mở HostSmith" trailing="⌘O" onClick={() => void panelOpenMain()} />
        <Row title="Cài đặt" trailing="⌘," onClick={() => void panelOpenMain("settings")} />
        <Row title="Thoát HostSmith" trailing="⌘Q" onClick={() => void panelQuit()} />
      </div>
    </div>
  );
}

/** A panel row: title, optional mono subtitle, trailing shortcut or status. */
function Row({
  title,
  subtitle,
  trailing,
  accent,
  danger,
  disabled,
  onClick,
}: {
  title: string;
  subtitle?: string;
  trailing?: string;
  accent?: boolean;
  danger?: boolean;
  disabled?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className="flex min-h-8 items-center gap-3 rounded-md px-3 py-1 text-left transition-colors hover:bg-accent disabled:pointer-events-none disabled:opacity-50"
    >
      <span className="flex min-w-0 flex-1 flex-col">
        <span className={cn("truncate", danger && "text-destructive", accent && "text-primary")}>{title}</span>
        {subtitle && <span className="truncate font-mono text-[11px] text-muted-foreground">{subtitle}</span>}
      </span>
      {trailing && (
        <span className={cn("shrink-0 text-xs", danger ? "text-destructive" : "text-muted-foreground")}>
          {trailing}
        </span>
      )}
    </button>
  );
}

function GroupLabel({ children }: { children: React.ReactNode }) {
  return <p className="px-3 pt-1 pb-1 text-xs font-medium text-muted-foreground">{children}</p>;
}

function Empty({ children }: { children: React.ReactNode }) {
  return <p className="flex h-8 items-center px-3 text-xs text-muted-foreground">{children}</p>;
}

function Separator() {
  return <div className="mx-3 my-1.5 h-px shrink-0 bg-border" />;
}
