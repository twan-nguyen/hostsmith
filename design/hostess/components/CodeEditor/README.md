# CodeEditor

The monospace editor behind the hosts pane, the `.env` Raw view and the diff. You provide the text, the language and the pending changes; the editor provides the gutter, the colouring and the line states.

Four colours, and no more: `code-ip` for addresses and numeric literals, `code-key` for hostnames and variable names, `code-value` for values and quoted strings, `code-comment` for comments. A fifth colour would be decoration — this is a config file, not a program.

Line states are the whole grammar:

| Class | Means |
| --- | --- |
| `is-current` | The caret's line, on `accent-tint`. One at a time. |
| `is-off` | An entry Hostess has commented out, at `opacity-disabled`. |
| `is-added` | A pending addition, on `diff-add`. |
| `is-removed` | A pending removal, on `diff-remove`. |

Toggling an entry off comments it out; it never deletes the line. That is what makes a toggle safe to flip from the menu bar, and it must hold in every pane that writes a file.

Preserve the file exactly. Comments, blank lines, alignment whitespace and the trailing newline all survive a round trip through Hostess, whether the person edited in Raw or in the table. A diff that shows a line the person did not touch is a bug.

The gutter is `gutter-width`, `surface-sunken`, `ink-tertiary`, and not selectable. Blocks fold at their `# Hostess — block:` comment.

Nothing here writes on its own. The editor is dirty until `Apply`, and `⌘Z` undoes an applied write, not just the typing.

The editor's text size is set once in Settings and is independent of the system text size, because developers pick a size and keep it.
