# MenuBarPanel

The panel that drops from the status item. You provide the active profile, the toggles, the ports and the footer items; the panel provides the material, the width and the dismissal.

It is exactly `menubar-width` and never resizes. `radius-lg` corners, `surface-menubar` with a `hairline` edge and `shadow-popover`. The active profile sits in its own `surface-sidebar` card at the top, its name in the serif.

Four regions in a fixed order: the active profile with its `EnvChip`, up to eight quick toggles, up to three ports, then the footer. Separate them with `hs-menubar__sep`, never with whitespace alone.

**Nothing in the panel scrolls.** Eight toggles and three ports is the cap; anything beyond it collapses into "Show all in Hostess". A panel that scrolls has already failed.

Flipping a toggle keeps the panel open, because people flip two or three in a row. Selecting a footer item or a port dismisses it.

A toggle commits immediately, optimistically, and reverts with an inline `Banner` if the write fails — never a system alert. A production change is the one exception: it opens a confirmation sheet on the window and the panel dismisses.

Hover is `surface-hover` at `radius-sm`, inset from the panel edge. The text colour does not change.

Keyboard throughout: `↑` `↓` to move, `Space` to flip, `⏎` to open in the window, `Escape` to dismiss, and `⌘1`–`⌘8` for the toggles by position.

The panel holds no text editing, no file pickers and no settings. Those are the window's job.
