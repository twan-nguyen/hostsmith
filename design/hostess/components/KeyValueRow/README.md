# KeyValueRow

One variable in a `.env` file. You provide the key, the value, whether it is a secret, and its diff state.

The key is `mono-strong` in `code-key`, fixed at 200px so a column of keys reads as a column. The value is `mono-body` in `code-value` and takes the rest of the row.

**Mask by default, reveal on demand.** A key matching `*_KEY`, `*_SECRET`, `*_TOKEN`, `*_PASSWORD`, `*_DSN` or `*_CREDENTIALS` is masked to `ink-tertiary` bullets. Reveal asks for Touch ID or the login password, reveals one value, and re-masks after 30 seconds or when the row loses focus, whichever comes first. Never reveal a whole file at once, and never print a revealed value into a banner, a log or a window title.

A value kept in the Keychain shows `key.fill` before its reveal button; the file on disk holds a reference, not the secret.

Diff state uses `is-added` and `is-removed` against `.env.example` or the last applied version. A removed row keeps its text struck through in `code-comment` so the person can read what is going away before they apply.

A variable that exists in `.env.example` but not here is shown as a real row with a `Missing` value in `ink-tertiary` and an `Add` button. Missing variables are the most common cause of a broken afternoon, so they are visible in the table, not hidden behind a filter.

The table is a view of the file, not a replacement for it. Comments, ordering and blank lines survive; editing a value here rewrites exactly that one line.
