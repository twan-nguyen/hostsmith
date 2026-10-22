const OVERLAY = "[role=dialog], [role=alertdialog], [role=menu]";

/**
 * Whether a dialog, an alert or a menu is open, or `target` sits in one.
 * Keys then belong to it, not to the pane behind it. The target matters for
 * Esc: by the time a window listener sees it, the overlay may already have
 * started to close.
 */
export function overlayOpen(target?: EventTarget | null): boolean {
  if (target instanceof Element && target.closest(OVERLAY)) return true;
  return document.querySelector(OVERLAY) !== null;
}
