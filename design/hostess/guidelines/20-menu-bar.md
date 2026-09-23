# Menu bar and quick toggles

The panel is the part of Hostess people actually use. It opens from the status item, is exactly `menubar-width` wide, has `radius-lg` corners, `surface-menubar` over the blurred desktop, a `hairline` edge and `shadow-popover`. It opens in 120ms and dismisses on Escape, on click-outside, and after any action that is unambiguous.

## The status item

The icon is the SF Symbol `network`, template-rendered so macOS tints it for the menu bar — never a coloured bitmap.

Show state in the icon itself, because that is the only part of Hostess visible all day:

- No profile active → `network`, normal weight.
- A profile active → `network.badge.shield.half.filled`.
- Production active → the same symbol plus the profile's initial in a `danger` badge. Production must be visible without opening anything.
- A write failed → `exclamationmark.triangle.fill`.

Never animate the status item, and never show a number badge for anything routine.

## Panel layout, top to bottom

1. **Active profile.** One row, `text-headline` name plus an `EnvChip`. Tapping it opens the profile switcher inline — not a submenu, not the window.
2. **Quick toggles.** The heart of the panel: a `text-menu-label` group header, then up to eight `ListRow`s each with a label, a `text-caption` sublabel showing what it maps to (`127.0.0.1 → api.acme.test`), and a `Switch` on the trailing edge. A toggle flips instantly and the panel stays open, because people flip two or three in a row.
3. **Ports.** A group header with the count, then up to three `PortRow`s in compact form. More than three collapses to "Show all in Hostess".
4. **Footer.** `Open Hostess`, `Settings…`, `Quit`, each in `text-body` with its shortcut in `text-caption` `ink-secondary` on the trailing edge.

Separate groups with `hairline` inset by `space-5`, never with extra whitespace alone.

## Rules for toggles

A toggle writes to disk. Because the panel has no `Apply` button, a flip is committed immediately — so the toggle set must only contain actions that are instant, reversible, and safe to fire by accident. Anything that is not all three belongs in the window.

Apply optimistically: move the `Switch` at once, then do the work. If the write fails, return the switch to its previous position and show a `Banner` inside the panel — never a system alert, and never leave the switch in a state the file is not in.

A toggle that needs administrator rights shows `lock.fill` in `ink-secondary` after its label. The first flip in a session raises the prompt; the rest of the session does not.

Production is exempt from everything above. Switching a profile to Production, or flipping a toggle that points at a production host, opens a confirmation sheet on the window with the target named in `mono-body` — the panel never commits a production change on its own.

## Keyboard

The panel is fully keyboard-driven: a global shortcut opens it, `↑` `↓` move, `Space` flips the focused toggle, `⏎` opens the focused item in the window, `Escape` dismisses. `⌘1`–`⌘8` flip the toggles by position, and the position is shown in `text-caption` once the `⌘` key is held.

## What never goes in the panel

No text editing. No file pickers. No settings. No scrolling list longer than a screen — cap the panel at eight toggles and three ports, and send the rest to the window. A panel that scrolls has already failed.
