# EnvChip

Names an environment wherever one appears — a profile row, the menu bar header, an editor title, a confirmation sheet. You provide the name; the chip provides the colour.

The mapping is fixed and there are only three colours: `env-local` green, `env-staging` amber, `env-production` red. They are the same three hues as the status scale on purpose — green is safe, amber is care, red is production — so a colour never means two things in this app.

Any other environment is `hs-chip--neutral` and relies on its name. Do not invent a fourth hue.

Always show the word. The chip is `text-menu-label` sized, never an icon or a bare dot, because "which environment am I in" is the question the whole app exists to answer.

Use the tint variants inside a working pane, where the chip sits among other content. Use `hs-chip--solid` only in the menu bar header and a confirmation sheet, where the chip is the loudest thing and is meant to be.

A production chip anywhere in the window means the next destructive action opens a sheet. Do not show one decoratively.
