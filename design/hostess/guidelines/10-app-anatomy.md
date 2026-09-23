# App anatomy

Hostess is one window plus one menu bar panel. The window has a source list on the left and a single working pane on the right; there are no tabs inside the pane, and no second window except Settings and About.

## The source list

`Sidebar`, `sidebar-width` wide, on `surface-sidebar`. Four fixed groups, each labelled in `text-menu-label`, in this order:

| Group | Rows |
| --- | --- |
| **Profiles** | One row per profile — `Local`, `Staging`, `Production`, plus any the person adds. The active one carries an `EnvChip` on the trailing edge. |
| **Hosts** | `System hosts` (`/etc/hosts`), then one row per hosts group the person has created. |
| **Projects** | One row per folder the person has added, each expanding to the `.env*` files found inside it. |
| **Tools** | `Ports`, and anything else that is a utility rather than a file. |

A row is `row-height` tall, `space-5` of horizontal padding, an SF Symbol, a `text-body` title, and an optional trailing `StatusDot` or count in `text-caption`. Selection follows the rules in the brand book.

## The panes

**Hosts** — a `CodeEditor` on `surface-content` with a `gutter-width` gutter, one entry per line, syntax coloured with `code-ip`, `code-key` and `code-comment`. Above it a `Toolbar` holds the file path in `mono-caption`, a search field, and `Apply`. Blocks of entries can be folded and toggled as a unit; a toggled-off block is commented out in the file rather than deleted, so nothing is ever lost.

**Environment file** — a two-column table of `KeyValueRow`s: key in `mono-strong` coloured `code-key`, value in `mono-body` coloured `code-value`, masked to `ink-tertiary` when the key looks like a secret. A `SegmentedControl` in the toolbar switches between **Table**, **Raw** (the same `CodeEditor`) and **Diff** (against `.env.example` or the last applied version, using `diff-add` and `diff-remove`). Comments and blank lines survive every round trip through the table — that is a hard requirement, not a nicety.

**Profile** — the only pane that is a form. It lists what the profile changes when it is activated: hosts blocks on, hosts blocks off, which `.env` file is linked into each project, and optional shell exports. Each line is a `ListRow`. A `Banner` at the top states what activating will touch, counted: "Activating rewrites 14 hosts entries and swaps 2 .env files."

**Ports** — a table of `PortRow`s: port in `mono-caption`, process name, PID, the project it belongs to when Hostess can tell, and a trailing `Kill process` button in `danger`. Free ports are not listed; the table is what is *taken*.

**Secrets** — the same `KeyValueRow` table, but values live in the macOS Keychain and the file on disk holds a reference. Reveal requires Touch ID or the login password, reveals one value, and re-masks after 30 seconds or on blur, whichever comes first.

## Permissions

`/etc/hosts` needs administrator rights. Ask once, through a privileged helper installed on first use, and explain it in a `Banner` before the system prompt appears — never let macOS's own dialog be the first thing the person sees. A project folder needs a security-scoped bookmark: ask with the standard open panel, and say in the empty state why.

## Data safety

Every write to `/etc/hosts` or a `.env` file snapshots the previous content first. Keep the last 50 snapshots per file, show them in a history popover with a timestamp in `text-caption`, and make restore a single action. This is what makes the rest of the product safe to use quickly.
