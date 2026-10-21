import { openUrl } from "@tauri-apps/plugin-opener";
import { RotateCw } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { Banner, type BannerTone } from "@/components/banner";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { PaneEmpty } from "@/components/pane-empty";
import { Panel, PanelHeader } from "@/components/panel";
import { SegmentedControl } from "@/components/segmented-control";
import { Button } from "@/components/ui/button";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import {
  type ListeningPort,
  type ToastKind,
  portsKill,
  portsScan,
  portsStopContainer,
  toolsOpenInEditor,
  toolsOpenInTerminal,
  toolsRevealInFinder,
} from "@/lib/hosven";
import {
  type OwnerTone,
  attentionPort,
  filterPorts,
  localhostUrl,
  meaningfulCwd,
  ownerLabel,
  ownerTone,
  portDetail,
  portsSubtitle,
} from "@/lib/ports-view";
import { useHomeDir } from "@/lib/use-home-dir";
import { useToast } from "@/lib/use-toast";
import { cn } from "@/lib/utils";
import { knownRepos, useStore } from "@/store";

const GRID = { gridTemplateColumns: "72px minmax(0,1fr) 112px auto" };

const TOAST_TONE: Record<ToastKind, BannerTone> = { success: "success", error: "danger", info: "info" };

const DOT: Record<OwnerTone, string> = {
  ok: "bg-success",
  idle: "bg-muted-foreground/50",
  danger: "bg-destructive",
};

/** How long a row stays highlighted after "Hiện tiến trình" scrolls to it. */
const HIGHLIGHT_MS = 2400;

/** A stop to confirm: SIGTERM for a process nobody knows, SIGKILL when asked or when SIGTERM was not enough. */
type PendingStop = { port: ListeningPort; force: boolean; afterTerm: boolean };

/** Re-renders every second, so "N giây trước" counts up. */
function useNow(intervalMs = 1000) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), intervalMs);
    return () => window.clearInterval(timer);
  }, [intervalMs]);
  return now;
}

/** The TCP ports something listens on, who holds each, and ways to free it. */
export function PortsPane() {
  const { ports: snapshot, tools, portsFilter, setPortsFilter, run } = useStore();
  const home = useHomeDir();
  const now = useNow();
  const [toast, dismissToast] = useToast(snapshot?.toast ?? null);
  const [scanning, setScanning] = useState(false);
  const [busy, setBusy] = useState<Set<string>>(new Set());
  const [pending, setPending] = useState<PendingStop | null>(null);
  const [highlighted, setHighlighted] = useState<string | null>(null);
  const rows = useRef(new Map<string, HTMLDivElement>());
  const highlightTimer = useRef<number | undefined>(undefined);

  const ports = snapshot?.ports ?? [];
  const visible = filterPorts(ports, portsFilter);
  const attention = portsFilter === "all" ? attentionPort(ports) : null;
  const isScanning = scanning || Boolean(snapshot?.scanning);

  // Ports belong to whichever repo Hosven knows at the time of the call.
  const repos = () => knownRepos(useStore.getState().env);

  async function scan() {
    setScanning(true);
    try {
      await run(portsScan(repos()));
    } finally {
      setScanning(false);
    }
  }

  // Scan when the pane appears; ⌘R scans again.
  const scanRef = useRef(scan);
  scanRef.current = scan;
  useEffect(() => {
    void scanRef.current();
    const onKeyDown = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey) || e.key.toLowerCase() !== "r" || e.shiftKey || e.altKey) return;
      // Always keep ⌘R from reloading the window.
      e.preventDefault();
      if (document.querySelector("[role=dialog], [role=alertdialog], [role=menu]")) return;
      void scanRef.current();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.clearTimeout(highlightTimer.current);
    };
  }, []);

  function setRowBusy(port: ListeningPort, on: boolean) {
    setBusy((ids) => {
      const next = new Set(ids);
      if (on) next.add(port.id);
      else next.delete(port.id);
      return next;
    });
  }

  /** Sends the signal; the backend rescans and reports in a toast. */
  async function stop(port: ListeningPort, force: boolean) {
    setRowBusy(port, true);
    try {
      const outcome = await run(portsKill(port.pid, force, repos()));
      if (outcome === "stillRunning" && !force) setPending({ port, force: true, afterTerm: true });
    } finally {
      setRowBusy(port, false);
    }
  }

  async function stopContainer(port: ListeningPort, name: string) {
    setRowBusy(port, true);
    try {
      await run(portsStopContainer(name, repos()));
    } finally {
      setRowBusy(port, false);
    }
  }

  function reveal(port: ListeningPort) {
    rows.current.get(port.id)?.scrollIntoView({ block: "center", behavior: "smooth" });
    setHighlighted(port.id);
    window.clearTimeout(highlightTimer.current);
    highlightTimer.current = window.setTimeout(() => setHighlighted(null), HIGHLIGHT_MS);
  }

  const openLocalhost = (port: ListeningPort) => void run(openUrl(localhostUrl(port.port)));

  return (
    <Panel>
      <PanelHeader>
        <div className="flex min-w-0 flex-1 items-baseline gap-2.5">
          <h1 className="shrink-0 font-medium">Ports</h1>
          <span className="truncate text-xs text-muted-foreground tabular-nums" data-testid="ports-subtitle">
            {portsSubtitle(ports, snapshot?.scannedAt ?? null, now)}
          </span>
        </div>
        <SegmentedControl
          label="Lọc cổng"
          value={portsFilter}
          onChange={setPortsFilter}
          options={[
            { value: "all", label: "Tất cả", testId: "ports-filter-all" },
            { value: "repos", label: "Repo của bạn", testId: "ports-filter-repos" },
            { value: "system", label: "Hệ thống", testId: "ports-filter-system" },
          ]}
        />
        <Button
          size="sm"
          variant="outline"
          title="Quét lại (⌘R)"
          data-testid="ports-rescan"
          disabled={isScanning}
          onClick={() => void scan()}
        >
          <RotateCw className={cn(isScanning && "animate-spin")} />
          {isScanning ? "Đang quét" : "Quét lại"}
        </Button>
      </PanelHeader>

      {toast ? (
        <div className="shrink-0 px-5 pt-4">
          <Banner tone={TOAST_TONE[toast.kind]} title={toast.message} onDismiss={dismissToast} />
        </div>
      ) : (
        attention && (
          <div className="shrink-0 px-5 pt-4">
            <Banner
              tone="danger"
              title={`${attention.process} trên cổng ${attention.port} không thuộc repo nào Hosven biết`}
              actions={[{ label: "Hiện tiến trình", onClick: () => reveal(attention) }]}
            >
              Dừng nó sẽ mở hộp thoại xác nhận nêu tên tiến trình và pid. Hosven gửi SIGTERM trước.
            </Banner>
          </div>
        )
      )}

      <div className="min-h-0 flex-1 overflow-auto">
        {visible.length === 0 ? (
          <PaneEmpty
            title={isScanning ? "Đang quét" : "Không có cổng nào đang lắng nghe"}
            detail={
              portsFilter === "repos"
                ? "Không có tiến trình nào chạy từ repo Hosven biết đang giữ cổng."
                : "Hosven liệt kê các cổng đang bị chiếm, không liệt kê cổng trống."
            }
          >
            <Button variant="outline" disabled={isScanning} onClick={() => void scan()}>
              Quét lại
            </Button>
          </PaneEmpty>
        ) : (
          <>
            <div
              className="sticky top-0 z-10 grid h-8 items-center gap-4 border-b border-border bg-card px-5 text-xs text-muted-foreground"
              style={GRID}
            >
              <span>Cổng</span>
              <span>Tiến trình</span>
              <span>Chủ sở hữu</span>
              <span className="sr-only">Thao tác</span>
            </div>
            {visible.map((port) => {
              const cwd = meaningfulCwd(port);
              const rowBusy = busy.has(port.id);
              const tone = ownerTone(port);
              return (
                <ContextMenu key={port.id}>
                  <ContextMenuTrigger asChild>
                    <div
                      ref={(el) => {
                        if (el) rows.current.set(port.id, el);
                        else rows.current.delete(port.id);
                      }}
                      data-testid={`port-row-${port.port}`}
                      className={cn(
                        "grid min-h-12 animate-fade-in items-center gap-4 border-b border-border/50 px-5 py-1.5 transition-colors duration-300 hover:bg-accent data-[state=open]:bg-accent",
                        highlighted === port.id && "bg-destructive/10 hover:bg-destructive/15",
                      )}
                      style={GRID}
                    >
                      <span className="flex h-6 w-fit min-w-14 items-center justify-center rounded-md bg-muted px-2 font-mono text-xs font-medium tabular-nums">
                        {port.port}
                      </span>
                      <span className="flex min-w-0 flex-col">
                        <span className="truncate font-medium" title={port.process}>
                          {port.process}
                        </span>
                        <span
                          className="truncate font-mono text-xs text-muted-foreground"
                          title={port.cwd ?? undefined}
                        >
                          {portDetail(port, home)}
                        </span>
                      </span>
                      <span className="flex items-center gap-2 text-muted-foreground">
                        <span className={cn("size-2 shrink-0 rounded-full", DOT[tone])} />
                        {ownerLabel(port.owner)}
                      </span>
                      <span className="flex items-center justify-end gap-1.5">
                        {port.owner.kind === "yours" && (
                          <>
                            <Button size="xs" variant="outline" onClick={() => openLocalhost(port)}>
                              Mở
                            </Button>
                            <Button size="xs" variant="destructive" disabled={rowBusy} onClick={() => void stop(port, false)}>
                              Dừng
                            </Button>
                          </>
                        )}
                        {port.owner.kind === "container" && (
                          <Button
                            size="xs"
                            variant="outline"
                            disabled={rowBusy || !port.owner.name}
                            title={port.owner.name ? `docker stop ${port.owner.name}` : "Không rõ tên container"}
                            onClick={() => {
                              if (port.owner.kind === "container" && port.owner.name) {
                                void stopContainer(port, port.owner.name);
                              }
                            }}
                          >
                            Dừng
                          </Button>
                        )}
                        {port.owner.kind === "unknown" && (
                          <Button
                            size="xs"
                            variant="destructive"
                            disabled={rowBusy}
                            onClick={() => setPending({ port, force: false, afterTerm: false })}
                          >
                            Dừng
                          </Button>
                        )}
                      </span>
                    </div>
                  </ContextMenuTrigger>
                  <ContextMenuContent>
                    <ContextMenuItem onSelect={() => openLocalhost(port)}>Mở localhost:{port.port}</ContextMenuItem>
                    {cwd && (
                      <>
                        <ContextMenuItem onSelect={() => void run(toolsRevealInFinder(cwd))}>
                          Hiện thư mục trong Finder
                        </ContextMenuItem>
                        {tools?.terminal && (
                          <ContextMenuItem onSelect={() => void run(toolsOpenInTerminal(cwd))}>
                            Mở thư mục trong {tools.terminal.name}
                          </ContextMenuItem>
                        )}
                        {tools?.editor && (
                          <ContextMenuItem onSelect={() => void run(toolsOpenInEditor(cwd))}>
                            Mở thư mục bằng {tools.editor.name}
                          </ContextMenuItem>
                        )}
                      </>
                    )}
                    <ContextMenuSeparator />
                    <ContextMenuItem
                      variant="destructive"
                      disabled={rowBusy}
                      onSelect={() => setPending({ port, force: true, afterTerm: false })}
                    >
                      Buộc dừng (SIGKILL)
                    </ContextMenuItem>
                  </ContextMenuContent>
                </ContextMenu>
              );
            })}
          </>
        )}
      </div>

      <ConfirmDialog
        open={pending !== null}
        title={pending?.force ? "Buộc dừng tiến trình?" : "Dừng tiến trình?"}
        description={
          pending && (
            <span className="flex flex-col gap-2">
              <span className="font-mono text-xs text-foreground">
                {pending.port.process} (pid {pending.port.pid}) · cổng {pending.port.port}
              </span>
              <span>
                {pending.force
                  ? pending.afterTerm
                    ? "Tiến trình vẫn chạy sau SIGTERM. SIGKILL dừng nó ngay và không cho nó lưu gì cả."
                    : "SIGKILL dừng tiến trình ngay và không cho nó lưu gì cả."
                  : "Hosven không biết tiến trình này thuộc project nào. Nó nhận SIGTERM và có thể mất dữ liệu chưa lưu."}
              </span>
            </span>
          )
        }
        confirmLabel={pending?.force ? "Buộc dừng" : "Dừng tiến trình"}
        onConfirm={() => pending && void stop(pending.port, pending.force)}
        onClose={() => setPending(null)}
      />
    </Panel>
  );
}
