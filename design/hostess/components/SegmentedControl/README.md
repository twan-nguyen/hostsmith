# SegmentedControl

Switches between views of the same content. You provide two to four segments and the selected index.

Use it for the `.env` pane's **Table / Raw / Diff**, and for scoping a list. Never use it as a row of buttons: every segment must show the same data a different way, and switching must never write anything.

Two to four segments, each one or two words, sentence case. If you need five, you need a `Sidebar` row or a pop-up menu instead.

The active segment is a raised `surface-raised` tile with `shadow-control` and `ink-primary` — not the accent, which is reserved for actions. Keep the control at `control-height-md`, inside a `surface-sunken` track at `radius-sm`, so it reads as one control rather than three buttons.

Remember the selection per pane and restore it when the person comes back — a developer who works in Raw expects Raw next time.
