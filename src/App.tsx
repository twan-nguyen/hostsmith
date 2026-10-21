import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";

import { CommandPalette } from "@/components/command-palette";
import { Notice } from "@/components/notice";
import { PaneRouter } from "@/components/pane-router";
import { PendingSwitchDialog } from "@/components/pending-switch-dialog";
import { Sidebar } from "@/components/sidebar";
import { TooltipProvider } from "@/components/ui/tooltip";
import { hostsRedo, hostsUndo } from "@/lib/hosven";
import { parseSelection } from "@/lib/selection";
import { useStore } from "@/store";

const darkScheme = window.matchMedia("(prefers-color-scheme: dark)");

/** Inputs, textareas and editors keep their own undo history. */
function editableFocused() {
  const el = document.activeElement;
  return (
    el instanceof HTMLInputElement ||
    el instanceof HTMLTextAreaElement ||
    (el instanceof HTMLElement && el.isContentEditable)
  );
}

/** Runs a menu action the window owns. */
function onMenu(id: string) {
  const state = useStore.getState();
  const hostsPane = state.selection.kind === "systemHosts" || state.selection.kind === "profile" || state.selection.kind === "hostsTag";
  switch (id) {
    case "undo":
    case "redo":
      if (editableFocused()) document.execCommand(id);
      else if (hostsPane) void state.run(id === "undo" ? hostsUndo() : hostsRedo());
      break;
    case "find":
      state.requestFind();
      break;
    case "toggle-sidebar":
      state.toggleSidebar();
      break;
    case "settings":
      if (state.selection.kind === "settings") state.closeSettings();
      else state.openSettings();
      break;
  }
}

function App() {
  const init = useStore((s) => s.init);
  const settings = useStore((s) => s.settings);
  const sidebarCollapsed = useStore((s) => s.sidebarCollapsed);

  useEffect(() => {
    void init();
    const unlisten = [
      listen<string>("menu", (e) => onMenu(e.payload)),
      listen<string>("navigate", (e) => useStore.getState().select(parseSelection(e.payload))),
    ];
    return () => unlisten.forEach((u) => void u.then((f) => f()));
  }, [init]);

  // Light, dark, or whatever the system uses.
  const mode = settings?.appearanceMode ?? "system";
  useEffect(() => {
    const apply = () =>
      document.documentElement.classList.toggle("dark", mode === "dark" || (mode === "system" && darkScheme.matches));
    apply();
    darkScheme.addEventListener("change", apply);
    return () => darkScheme.removeEventListener("change", apply);
  }, [mode]);

  // The window is transparent; the canvas tint decides how much blur shows through.
  const transparency = settings?.transparency ?? 0;
  useEffect(() => {
    const root = document.documentElement;
    root.classList.toggle("vibrancy", transparency > 0);
    root.style.setProperty("--canvas-opacity", `${100 - transparency}%`);
  }, [transparency]);

  // Esc leaves Settings, unless an editor or a dialog wants it.
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const { selection, paletteOpen, closeSettings } = useStore.getState();
      if (
        event.key === "Escape" &&
        selection.kind === "settings" &&
        !paletteOpen &&
        !(event.target instanceof Element && event.target.closest(".cm-editor, [role=dialog], [role=alertdialog]"))
      ) {
        closeSettings();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  return (
    <TooltipProvider>
      {/* The gutter around the panel doubles as a window drag handle. */}
      <div data-tauri-drag-region className="app-canvas flex h-screen py-2 pr-2">
        <Sidebar />
        <div className={sidebarCollapsed ? "flex min-w-0 flex-1 flex-col pl-2" : "flex min-w-0 flex-1 flex-col"}>
          {/* With the sidebar hidden, the traffic lights need the strip above the panel. */}
          {sidebarCollapsed && <div data-tauri-drag-region className="h-8 shrink-0" />}
          <PaneRouter />
        </div>
      </div>
      <CommandPalette />
      <PendingSwitchDialog />
      <Notice />
    </TooltipProvider>
  );
}

export default App;
