# ListRow

The workhorse row: a glyph, a title, a monospace sublabel, and one trailing control. You provide all four; the row provides the rhythm.

The sublabel is the row's reason for existing. It shows what the title actually maps to — `127.0.0.1 → api.acme.test`, `~/work/acme/web/.env.local`, `0.0.0.0 → 180 entries` — in `mono-caption` `ink-secondary`, truncated at the end. A row without a sublabel is usually a row that is hiding something.

Exactly one trailing control: a `Switch`, a `StatusDot`, a count, or a small `Button`. Two trailing controls means the row is two rows.

The title never wraps. If it is too long, truncate it and put the full value in the sublabel.

A row whose action needs administrator rights shows `lock.fill` in the glyph slot and says so in the sublabel, rather than failing at the moment of the press.

`is-disabled` dims the whole row, label and glyph together, at `opacity-disabled`. Never recolour a disabled row and never hide it — a person needs to see that the thing exists and why it is unavailable.

Rows live inside one `hs-pane` card, separated by a `hairline`, and hover to `surface-hover`; the text colour does not change on hover. A glyph that needs weight sits in an `hs-row__glyph-well`, a `radius-sm` tile on `surface-sunken`.
