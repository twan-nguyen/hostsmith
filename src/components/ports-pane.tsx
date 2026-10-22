import { openUrl } from "@tauri-apps/plugin-opener";
import { Cable, RotateCw } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useShallow } from "zustand/react/shallow";

import { ConfirmDialog } from "@/components/confirm-dialog";
import { PaneEmpty } from "@/components/pane-empty";
import { Panel, PanelHeader } from "@/components/panel";
import { SegmentedControl } from "@/components/segmented-control";
import { StatusIcon, type StatusTone } from "@/components/status-icon";
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
  portsKill,
  portsScan,
  portsStopContainer,
  toolsOpenInEditor,
  toolsOpenInTerminal,
  toolsRevealInFinder,
} from "@/lib/hostsmith";
import {
  type OwnerTone,
  filterPorts,
  localhostUrl,
  meaningfulCwd,
  ownerLabel,
  ownerTone,
  portDetail,
  portsSubtitle,
} from "@/lib/ports-view";
import { overlayOpen } from "@/lib/overlay";
import { useHomeDir } from "@/lib/use-home-dir";
import { cn } from "@/lib/utils";
import { useStore } from "@/store";

// Fixed widths, so every row lines up whether or not its actions are showing.
const GRID = { gridTemplateColumns: "64px minmax(0,1fr) minmax(120px,200px) 104px" };

const TONE: Record<OwnerTone, StatusTone> = { project: "on", container: "muted", unknown: "off" };

/** A stop to confirm: SIGTERM first, SIGKILL when asked or when SIGTERM was not enough. */
type PendingStop = { port: ListeningPort; force: boolean; afterTerm: boolean };

/**
 * "Quét N giây trước". It ticks every second on its own, so
 * the rows under it do not render again with it.
 */
function PortsSubtitle({ scannedAt }: { scannedAt: number | null }) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, []);
  return (
    <span className="truncate text-xs text-muted-foreground tabular-nums" data-testid="ports-subtitle">
      {portsSubtitle(scannedAt, now)}
    </span>
  );
}

/** The TCP ports something listens on, who holds each, and ways to free it. */
export function PortsPane() {
  const { snapshot, tools, portsFilter, setPortsFilter, run } = useStore(
    useShallow((s) => ({
      snapshot: s.ports,
      tools: s.tools,
      portsFilter: s.portsFilter,
      setPortsFilter: s.setPortsFilter,
      run: s.run,
    })),
  );
  const home = useHomeDir();
  const [scanning, setScanning] = useState(false);
  const [busy, setBusy] = useState<Set<string>>(new Set());
  const [pending, setPending] = useState<PendingStop | null>(null);

  const ports = snapshot?.ports ?? [];
  const visible = filterPorts(ports, portsFilter);
  const isScanning = scanning || Boolean(snapshot?.scanning);

  async function scan() {
    setScanning(true);
    try {
      await run(portsScan());
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
      if (overlayOpen()) return;
      void scanRef.current();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
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
      const outcome = await run(portsKill(port.pid, force));
      if (outcome === "stillRunning" && !force) setPending({ port, force: true, afterTerm: true });
    } finally {
      setRowBusy(port, false);
    }
  }

  async function stopContainer(port: ListeningPort, name: string) {
    setRowBusy(port, true);
    try {
      await run(portsStopContainer(name));
    } finally {
      setRowBusy(port, false);
    }
  }

  const openLocalhost = (port: ListeningPort) => void run(openUrl(localhostUrl(port.port)));

  return (
    <Panel>
      <PanelHeader>
        <span className="text-muted-foreground [&_svg]:size-4">
          <Cable />
        </span>
        <h1 className="shrink-0 font-medium">Cổng</h1>
        <span className="text-muted-foreground tabular-nums">{visible.length}</span>
        <div className="ml-auto flex min-w-0 items-center gap-3">
          <PortsSubtitle scannedAt={snapshot?.scannedAt ?? null} />
        </div>
        <SegmentedControl
          label="Lọc cổng"
          value={portsFilter}
          onChange={setPortsFilter}
          options={[
            { value: "projects", label: "Dự án", testId: "ports-filter-projects" },
            { value: "user", label: "Người dùng", testId: "ports-filter-user" },
            { value: "system", label: "Hệ thống", testId: "ports-filter-system" },
          ]}
        />
        <Button
          size="icon-sm"
          variant="ghost"
          aria-label="Quét lại"
          title="Quét lại (⌘R)"
          data-testid="ports-rescan"
          disabled={isScanning}
          onClick={() => void scan()}
          className="text-muted-foreground"
        >
          <RotateCw className={cn(isScanning && "animate-spin")} />
        </Button>
      </PanelHeader>

      <div className="min-h-0 flex-1 overflow-auto">
        {visible.length === 0 ? (
          <PaneEmpty
            title={isScanning ? "Đang quét" : "Không có cổng nào đang lắng nghe"}
            detail={
              portsFilter === "projects"
                ? "Không có tiến trình nào chạy từ một dự án git đang giữ cổng."
                : "HostSmith liệt kê các cổng đang bị chiếm, không liệt kê cổng trống."
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
              <span>Thuộc về</span>
              <span aria-label="Thao tác" />
            </div>
            {visible.map((port) => {
              const cwd = meaningfulCwd(port);
              const rowBusy = busy.has(port.id);
              const tone = ownerTone(port);
              return (
                <ContextMenu key={port.id}>
                  <ContextMenuTrigger asChild>
                    <div
                      data-testid={`port-row-${port.port}`}
                      className="group grid min-h-12 animate-fade-in items-center gap-4 border-b border-border/50 px-5 py-1.5 transition-colors duration-100 hover:bg-accent data-[state=open]:bg-accent"
                      style={GRID}
                    >
                      <span className="font-mono text-xs font-medium tabular-nums">
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
                      <span
                        className={cn("flex min-w-0 items-center gap-2", tone !== "project" && "text-muted-foreground")}
                        title={tone === "container" ? "Container" : tone === "project" ? "Dự án git" : undefined}
                      >
                        <StatusIcon tone={TONE[tone]} />
                        <span className="truncate">{ownerLabel(port.owner)}</span>
                      </span>
                      {/* Actions show on the row under the pointer, as in a list of mail. */}
                      <span
                        className={cn(
                          "flex items-center justify-end gap-1.5 opacity-0 transition-opacity duration-100 group-hover:opacity-100 focus-within:opacity-100",
                          rowBusy && "opacity-100",
                        )}
                      >
                        {port.owner.kind === "yours" && (
                          <>
                            <Button size="xs" variant="outline" onClick={() => openLocalhost(port)}>
                              Mở
                            </Button>
                            <Button
                              size="xs"
                              variant="destructive"
                              disabled={rowBusy}
                              onClick={() => setPending({ port, force: false, afterTerm: false })}
                            >
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
                  : pending.port.owner.kind === "yours"
                    ? `Tiến trình của dự án ${pending.port.owner.name} nhận SIGTERM để tự dừng.`
                    : "HostSmith không biết tiến trình này thuộc dự án nào. Nó nhận SIGTERM và có thể mất dữ liệu chưa lưu."}
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
