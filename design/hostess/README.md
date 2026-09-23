Hostess is a macOS menu bar utility for developers. It edits the system hosts file and the `.env` files inside a project, groups them into switchable profiles, and shows what is holding a port. It is a tool people reach for twenty times a day and keep open for months, so it behaves like a system utility, not like a web app in a window.

Three rules sit above everything else in this system.

**It must feel like the Claude desktop app.** A warm ivory ground, warm greys, a serif for titles and a quiet sans for everything else, hairline borders instead of shadows, and one clay accent used so rarely that it still means something. Generous space; nothing shouts. Do not import a web component library, do not add a second accent, and do not draw a shadow where a hairline will do.

**It edits files people can break.** `/etc/hosts` needs `sudo`; a wrong `.env` takes down a developer's afternoon. Every write is previewed, every destructive action is named in full, and every change is reversible from inside the app.

**The menu bar is the product.** The window is where things are configured; the menu bar panel is where they are used. If a task cannot be done from the panel in two clicks, it is the wrong shape. See the *Menu bar* section.

## Content fundamentals

Write in sentence case. Title Case belongs only to menu titles macOS itself capitalises (`File`, `Edit`, `Window`).

Address the person as *you* and the app as *Hostess*, never *we*. Write "Hostess needs administrator access to write `/etc/hosts`", not "We need permission".

Name the real thing. Say `/etc/hosts`, `.env.local`, `port 5173`, `node (pid 41288)` — set every one of those in `mono-caption` or `mono-body`. Never write "the configuration file" when you can write the path.

Buttons are verbs that repeat the sentence's action: `Apply`, `Activate`, `Reveal`, `Kill process`, `Discard changes`. Never `OK` on a destructive sheet; never `Yes` / `No`.

State results as facts, in the past tense, with a number: "Applied 12 entries to /etc/hosts", "3 variables missing from .env.example", "Port 5173 released". Never "Success!" and never an exclamation mark anywhere in the product.

Errors say what failed, why, and what to do next, in that order, in `text-callout`: "Could not write /etc/hosts — administrator access was denied. Try again and approve the prompt." Never surface a raw `errno`.

No emoji anywhere in the interface. Status is a coloured `StatusDot` plus a word.

Keep secrets out of copy. Never print a secret value in a banner, a notification, a log line or a window title — refer to it by key: "STRIPE_SECRET_KEY was updated".

## Visual foundations

### Colour

Ground the window in `surface-window` — ivory, never pure white for a whole pane. The source list sits one warm step lower on `surface-sidebar` with a `hairline` on its trailing edge. Content that needs a container — the editor, a table, a list group — is a `surface-content` white card with a `hairline` border and `radius-md`; content that does not need one sits directly on the ivory. Use `surface-sunken` for wells the person types into or reads code out of: the editor gutter, a code block, the track of a segmented control. Hover and selection are `surface-hover`, a warm grey at 10%, on every row and menu item.

Use exactly one accent, `accent` — deep clay. It appears on the primary button, on links, on the glyph of the selected sidebar row and on the on state of a `Switch` (as `accent-bright`), and nowhere else. The selected sidebar row itself is `surface-hover` with a medium-weight label, the way the Claude app marks the current conversation — the accent is not spent on selection. Do not tint a surface with `accent` — use `accent-tint`, and use it for exactly one thing per view.

Never let colour carry meaning alone. `success`, `warning` and `danger` always appear with a word or an SF Symbol beside them, because green and red are the pair colour-blind developers lose first.

Environment colour is a fixed mapping — `env-local`, `env-staging`, `env-production` — and it is the same three hues as the status scale on purpose: green is safe, amber is care, red is production. Never invent a fourth colour for a fourth environment; use `ink-secondary` on `surface-sunken` and rely on the name.

Separate things with `hairline`, never with a shadow: around every card, under the toolbar, between rows, along the sidebar. A shadow appears only under something that opens over the window (`shadow-popover`) and, barely, under a switch knob (`shadow-control`). Nothing in the working window casts a shadow.

Secondary buttons and text fields are outlined with `hairline-strong` on a white fill. The two controls that must be findable with no fill at all — the off state of a switch, and an invalid or focused text field — draw `border-control`, the token that holds 3:1 in both themes and may carry meaning on its own.

Both themes ship together and both are checked: every text token's usage note names the grounds it is allowed on, and each of those pairs holds at least 4.5:1 in Light and in Dark. If you add a colour, add the note and the check with it.

### Type

Set everything in `text-body` unless a rule says otherwise. `text-headline` is for the label of a toggle row or a table header; `text-title` — the serif — for the title of every pane, a sheet title and a Settings section; `text-large-title` for onboarding, the About window and an empty state. The serif is the only editorial note in the app: never set body copy or a button in it.

Set anything a developer could paste into a terminal in the mono group — hostnames, IPs, variable names, values, paths, ports, PIDs. A hostname in `text-body` is a bug. Use `mono-body` in an editor, `mono-strong` for the key half of a key/value row, `mono-caption` for paths and inline chips.

Put metadata in `text-caption` and colour it `ink-secondary`, or `ink-tertiary` when it is a placeholder or a masked value. Never put metadata in `text-body` to make it easier to read; make the row taller instead.

Nothing is uppercase. Group labels in the sidebar and the panel are `text-menu-label` in sentence case and `ink-secondary`.

### Spacing and layout

Pad the content area with `pane-inset` and leave `pane-inset` between stacked cards. Pad a list row with `space-6` horizontally and `space-5` vertically, a chip with `space-4`. Put `space-4` between controls in a row and `space-7` between sections. When in doubt, add space rather than a line.

Rows are `row-height` tall and single-line. A row that needs a second line gets a `text-caption` subtitle and grows — it never wraps its title.

Controls come in three heights only: `control-height-sm` inside a table row, `control-height-md` everywhere by default, `control-height-lg` in the toolbar and for the default button of a sheet.

The sidebar is `sidebar-width`, resizable between 200px and 320px. The menu bar panel is exactly `menubar-width` and never resizes.

### States

Hover on a row: fill it with `surface-hover` at `radius-sm`, in both themes — never shift the text colour on hover.

Selection in the sidebar: `surface-hover` fill, the label at weight 500, the glyph in `accent`. In a table: `accent-tint` fill with `ink-primary` text. Dim toolbar glyphs and the selection to `opacity-inactive-window` when the window is not key, the way AppKit does.

Focus: a solid `focus-ring` ring, `focus-ring-width` thick, drawn 2px outside the control and following its capsule. It is never removed and never replaced by a colour change.

Disabled: `opacity-disabled` on the whole control including its label and glyph. Never recolour a disabled control.

Unsaved work: mark the document title with a dot and enable `Apply`; never auto-write `/etc/hosts` or a `.env` file. Nothing reaches disk without a press.

Destructive: `danger` text on a bordered button, not a `danger` fill. A `danger` fill is reserved for the confirm button inside a sheet, where it is the only filled control.

### Radii, depth and motion

Buttons, fields, sidebar rows, menu items and the segmented control share `radius-sm`; cards, banners and the editor take `radius-md`; the panel, popovers and sheets `radius-lg`. Chips, status dots and the switch are the only pills. Nothing is rounder than that — the Claude app is rounded, not bubbly.

Depth is almost flat. The window ground is level 0; cards are level 0 with a border; only what opens over the window is level 1 and carries `shadow-popover`.

Motion is quiet: a popover fades and rises 4px over 150ms; a sheet slides over 200ms; a switch knob eases over 160ms; a status change cross-fades over 80ms. Nothing bounces, nothing loops. Respect Reduce Motion by cutting straight to the end state.

## App icon and mark

The mark is an **H whose crossbar is a switch**: two capsule stems, a translucent track between them, and a solid knob sitting at the on position. It is a monogram, a toggle and a bridge between two hosts at once, and it is the only picture Hostess ever draws of itself. It is drawn in ivory on a flat clay ground — no gloss, no gradient beyond a whisper of tone.

The app icon ships in the four macOS Tahoe appearances, all in `assets/Logos/`: `hostess-app-icon-light.svg` (clay — the default), `-dark.svg` (deeper clay on slate), `-clear.svg` (translucent over the wallpaper) and `-tinted.svg` (system tint). The mark is identical in all four; only the glass under it changes. Never redraw the mark, never put it on a different shape, and never add a second element to it. Drop shadow and gloss are part of the icon, not the mark: below 32px the mark is drawn alone.

The menu bar status item is `hostess-menubar-template.svg` — the same geometry as a single black ink, which macOS tints as a template image. Its ink is `#000000` in the file so that AppKit can recolour it; never place it as a colour image. The active profile's environment colour sits on it as a 7px badge at the bottom-right (`env-staging` amber, `env-production` red; no badge for Local).

## Iconography

Hostess uses **SF Symbols** at the `medium` weight, matched to the text beside them — `Body` scale next to `text-body`, `Small` next to `text-caption`. No icon files ship with this system; they are drawn by macOS, so they inherit their colour from the label and need no separate token.

The working set, so the same idea always gets the same glyph:

| Idea | Symbol |
| --- | --- |
| Hosts entry | `network` |
| Environment file | `doc.text` |
| Profile | `square.stack.3d.up` |
| Port / process | `bolt.horizontal.circle` |
| Secret | `key.fill` (locked) · `eye` / `eye.slash` (reveal, hide) |
| Applied / healthy | `checkmark.circle.fill` |
| Drifted / needs attention | `exclamationmark.triangle.fill` |
| Failed / blocked | `xmark.octagon.fill` |
| Administrator action | `lock.fill` |
| Reveal in Finder | `folder` |
| Menu bar status item | the Hostess template glyph (`assets/Logos/hostess-menubar-template.svg`), never an SF Symbol |

Never use an emoji in place of a symbol, and never use a symbol as decoration — every glyph in Hostess is either a control or a status.

When a symbol carries meaning on its own, give it a text alternative for VoiceOver that says the state, not the shape: "Applied", not "green check".

The component previews in this system draw simplified inline SVG shapes where the real app draws an SF Symbol: SF Symbols are an AppKit resource and cannot render outside it. Read the table above, not a preview, for which glyph an idea gets.
