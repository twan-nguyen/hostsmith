# Switch

The on/off control for a single reversible thing — a hosts entry, a hosts block, a shell export. You provide the label, the current state and a handler; the switch owns nothing else.

A switch commits the moment it moves. There is no `Apply` behind it, so only put a switch on an action that is instant, reversible and safe to fire by accident. Anything else is a `Button`.

Move the knob optimistically, then write. If the write fails, move it back and show a `Banner` in the same container — never leave the switch in a state the file on disk is not in, and never raise a system alert from a menu bar panel.

The off state is carried by the knob position and the `border-control` outline, not by the `control-track` fill, which is deliberately quiet. Do not remove that border to make the control look cleaner.

Put the label before the switch and the switch on the trailing edge of its row. Never label a switch `On` / `Off`; label it with the thing it controls, and let a `text-caption` sublabel carry the detail (`127.0.0.1 → api.acme.test`).

A switch that needs administrator rights shows `lock.fill` in `ink-secondary` after its label. A switch that points at production does not commit on its own — it opens a confirmation sheet, and returns to its old position if the sheet is cancelled.

The accessibility label states the thing and the state together: "Redirect api.acme.test, on".
