import { useEffect, useRef } from "react";

/**
 * ⌘S for the visible pane. The code editor maps it itself, and an open
 * dialog or menu keeps it, so neither saves twice nor saves from behind.
 */
export function useSaveShortcut(onSave: () => void) {
  const latest = useRef(onSave);
  latest.current = onSave;

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (!e.metaKey || e.ctrlKey || e.altKey || e.shiftKey || e.key.toLowerCase() !== "s") return;
      if (e.defaultPrevented || (e.target instanceof Element && e.target.closest(".cm-editor"))) return;
      e.preventDefault();
      if (document.querySelector("[role=dialog], [role=alertdialog], [role=menu]")) return;
      latest.current();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);
}
