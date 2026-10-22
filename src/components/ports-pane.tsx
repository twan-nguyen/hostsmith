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
 * "Scanned N sec ago". It ticks every second on its own, so
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
        <h1 className="shrink-0 font-medium">Ports</h1>
        <span className="text-muted-foreground tabular-nums">{visible.length}</span>
        <div className="ml-auto flex min-w-0 items-center gap-3">
          <PortsSubtitle scannedAt={snapshot?.scannedAt ?? null} />
        </div>
        <SegmentedControl
          label="Filter ports"
          value={portsFilter}
          onChange={setPortsFilter}
          options={[
            { value: "projects", label: "Projects", testId: "ports-filter-projects" },
            { value: "user", label: "User", testId: "ports-filter-user" },
            { value: "system", label: "System", testId: "ports-filter-system" },
          ]}
        />
        <Button
          size="icon-sm"
          variant="ghost"
          aria-label="Rescan"
          title="Rescan (⌘R)"
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
            title={isScanning ? "Scanning" : "No ports are listening"}
            detail={
              portsFilter === "projects"
                ? "No process started from a git project holds a port."
                : "HostSmith lists the ports in use, not the free ones."
            }
          >
            <Button variant="outline" disabled={isScanning} onClick={() => void scan()}>
              Rescan
            </Button>
          </PaneEmpty>
        ) : (
          <>
            <div
              className="sticky top-0 z-10 grid h-8 items-center gap-4 border-b border-border bg-card px-5 text-xs text-muted-foreground"
              style={GRID}
            >
              <span>Port</span>
              <span>Process</span>
              <span>Owner</span>
              <span aria-label="Actions" />
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
                        title={tone === "container" ? "Container" : tone === "project" ? "Git project" : undefined}
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
                              Open
                            </Button>
                            <Button
                              size="xs"
                              variant="destructive"
                              disabled={rowBusy}
                              onClick={() => setPending({ port, force: false, afterTerm: false })}
                            >
                              Stop
                            </Button>
                          </>
                        )}
                        {port.owner.kind === "container" && (
                          <Button
                            size="xs"
                            variant="outline"
                            disabled={rowBusy || !port.owner.name}
                            title={port.owner.name ? `docker stop ${port.owner.name}` : "Unknown container name"}
                            onClick={() => {
                              if (port.owner.kind === "container" && port.owner.name) {
                                void stopContainer(port, port.owner.name);
                              }
                            }}
                          >
                            Stop
                          </Button>
                        )}
                        {port.owner.kind === "unknown" && (
                          <Button
                            size="xs"
                            variant="destructive"
                            disabled={rowBusy}
                            onClick={() => setPending({ port, force: false, afterTerm: false })}
                          >
                            Stop
                          </Button>
                        )}
                      </span>
                    </div>
                  </ContextMenuTrigger>
                  <ContextMenuContent>
                    <ContextMenuItem onSelect={() => openLocalhost(port)}>Open localhost:{port.port}</ContextMenuItem>
                    {cwd && (
                      <>
                        <ContextMenuItem onSelect={() => void run(toolsRevealInFinder(cwd))}>
                          Reveal folder in Finder
                        </ContextMenuItem>
                        {tools?.terminal && (
                          <ContextMenuItem onSelect={() => void run(toolsOpenInTerminal(cwd))}>
                            Open folder in {tools.terminal.name}
                          </ContextMenuItem>
                        )}
                        {tools?.editor && (
                          <ContextMenuItem onSelect={() => void run(toolsOpenInEditor(cwd))}>
                            Open folder in {tools.editor.name}
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
                      Force stop (SIGKILL)
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
        title={pending?.force ? "Force stop process?" : "Stop process?"}
        description={
          pending && (
            <span className="flex flex-col gap-2">
              <span className="font-mono text-xs text-foreground">
                {pending.port.process} (pid {pending.port.pid}) · port {pending.port.port}
              </span>
              <span>
                {pending.force
                  ? pending.afterTerm
                    ? "The process is still running after SIGTERM. SIGKILL stops it at once and lets it save nothing."
                    : "SIGKILL stops the process at once and lets it save nothing."
                  : pending.port.owner.kind === "yours"
                    ? `The process of project ${pending.port.owner.name} gets SIGTERM so it can stop on its own.`
                    : "HostSmith does not know which project this process belongs to. It gets SIGTERM and may lose unsaved data."}
              </span>
            </span>
          )
        }
        confirmLabel={pending?.force ? "Force stop" : "Stop process"}
        onConfirm={() => pending && void stop(pending.port, pending.force)}
        onClose={() => setPending(null)}
      />
    </Panel>
  );
}
