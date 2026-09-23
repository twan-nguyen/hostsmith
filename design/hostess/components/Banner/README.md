# Banner

An inline message at the top of a pane, a sheet or the menu bar panel. You provide the level, a title, an optional detail line and up to two actions.

Four levels, each with its tint and its glyph: **info** for what an action is about to change, **success** for what it changed, **warning** for drift and missing variables, **danger** for a failure.

The title is the whole message, stated as a fact with a number — "Activating Staging rewrites 14 hosts entries and swaps 2 .env files", "Applied 12 entries to /etc/hosts". If a reader stops after the title they should still know what happened. The detail line in `text-callout` `ink-secondary` carries the specifics: the names, the reason, the next step.

An error banner says what failed, why, and what to do, in that order. Never show a raw `errno`, and never write "Something went wrong".

Actions are optional and there are at most two, the rightmost being the recovery: `Try again`, `Add all`, `Undo`. An informational banner that precedes a write carries the write's own button, so the person never has to look elsewhere for it.

A success banner is transient — it fades after 5 seconds unless it offers `Undo`, in which case it stays for the length of the undo window and then goes. Info, warning and danger banners stay until the condition clears.

Never use a banner for something a `StatusDot` in a row can say. One banner per pane; if two conditions are true, state the more severe one and put the other in the row it belongs to.
