# Interaction, risk and accessibility

## The three levels of risk

Hostess sorts every action into one of three levels, and the level decides the interface. Use this table rather than judging case by case.

| Level | What it is | Interface |
| --- | --- | --- |
| **Instant** | Reversible, scoped to one file Hostess owns, safe to fire by accident: flipping a hosts entry, switching between Local and Staging, revealing a value. | Commits immediately. No confirmation. An undo in the footer for 5 seconds. |
| **Deliberate** | Writes a file the person will notice: applying an edited `/etc/hosts`, swapping the `.env` linked into a project, restoring a snapshot. | Requires a press on a named button (`Apply`, `Restore`). A `Banner` above it states what will change, counted. |
| **Dangerous** | Touches production, deletes something, or kills a process Hostess did not start. | A sheet on the window. The target named in `mono-body`, the consequence in one `text-callout` sentence, and a `danger`-filled confirm button whose label is the verb. Cancel is the default. |

Never promote a Dangerous action into the menu bar panel, and never demote a Deliberate one to a toggle.

## Undo

`⌘Z` works in every editor and also undoes the last applied write — the file write, not just the text. That is the promise that lets people move fast, so it may not be quietly dropped in a pane that finds it hard to implement.

## Empty states

Every pane has one, and it is the pane's tutorial. An SF Symbol at 28px in `ink-tertiary`, a `text-title` line that names the thing, a `text-callout` sentence that says why it is empty, and one button that fixes it. No illustrations, no marketing copy.

- Projects, empty: "Add the folder of a project and Hostess will find its `.env` files." → `Add project…`
- Ports, empty: "No ports are in use by a project Hostess knows about." → `Scan all ports`
- Profiles, empty: "A profile groups hosts entries and `.env` files so you can switch them together." → `New profile`

## Progress

Anything under 500ms shows nothing. Anything longer shows an indeterminate spinner beside the control that started it, and the control stays disabled, at `opacity-disabled`, with its label unchanged — never swap a label to "Loading…".

Scanning ports and reading a large project are the two slow paths. Both stream results into the table as they arrive rather than blocking on a full result.

## Notifications

Hostess posts a system notification for exactly two things: a background write that failed, and a profile that changed because something other than the person changed it (a script, a rebooted machine). Everything else is in the app. Never notify on success.

## Accessibility

Every control has a label that says its state, not its shape: a switch is "Redirect api.acme.test, on", not "toggle". Status is announced as its word — `Applied`, `Drifted`, `Failed`.

Full keyboard access throughout: every control reachable with `Tab`, every list navigable with the arrow keys, the focus ring never suppressed.

Respect Increase Contrast by swapping `hairline` for `border-control` everywhere and dropping `shadow-control`. Respect Reduce Motion by cutting to the end state. Respect Reduce Transparency by replacing `surface-menubar` with the opaque `surface-raised`.

Text scales with the system text size setting; the mono editor scales independently through Settings, because developers set it once and keep it. Nothing in the layout may depend on a fixed text height.
