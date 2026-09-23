# Sidebar

The source list: a `sidebar-width` column on `surface-sidebar`, one warm step below the ground, with a `hairline` on its trailing edge — the Claude app's sidebar. You provide the four groups and their rows; the sidebar provides selection, hover and disclosure.

The four groups are fixed and always in this order — **Profiles**, **Hosts**, **Projects**, **Tools** — each labelled in `text-menu-label`. A person who has used Hostess once must find the same thing in the same place forever.

A row is `row-height` tall with an SF Symbol, a `text-body` name that truncates at the end, and an optional trailing count or `StatusDot` in `text-caption`. Child rows — a hosts block, a `.env` file inside a project — use `is-child` and the same height.

Selection is `surface-hover` with the label at weight 500 and the glyph in `accent` — quiet, the way the current conversation is marked in Claude. `is-selected-inactive` is the same fill without the accent glyph, for a window that is not key.

The trailing count is the number of entries, not the number of enabled ones. Enabled state lives in the pane, not here.

The sidebar never holds an action. No "+" row, no inline rename, no trash — those live in the toolbar, the context menu and the `⌘` shortcuts.

Resizable between 200px and 320px, and the width is remembered. Below 200px the column collapses entirely rather than shrinking the text.
