# Toolbar

A 60px strip on the ivory ground with a `hairline` underneath, the way a Claude conversation carries its title. You provide the title, the file path, the view controls and the pane's one default action.

Leading edge: the title in `text-title` — the serif — with the full path beside it in `mono-caption` `ink-secondary`. The path is not decoration — it is how a developer confirms they are editing the right file. Truncate it in the middle, never at the end, so the filename always survives.

Trailing edge, in this order: status, then view controls, then the default action. `Apply` is always last and always the rightmost thing in the window, so muscle memory finds it in every pane.

The toolbar holds exactly one `hs-btn--primary`, and only when there is something to apply. When there is nothing pending, leave the button enabled and let it report "Nothing to apply" — do not let the toolbar flicker between states as the person types.

Controls sit directly on the strip at `control-height-md`, separated by `space-4`; no grouping chrome. Dim every glyph to `opacity-inactive-window` when the window is not key.

No overflow menu and no customisation. If a pane needs more than four trailing controls, it is doing too much.
