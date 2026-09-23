# Button

A rounded push button at one of three heights, in four emphases — white with a `hairline-strong` border by default, clay when it is the one default action. You provide the label and the click handler; the button provides nothing but its own appearance.

Label a button with the verb of the sentence it completes — `Apply`, `Activate`, `Kill process`, `Add project…` — and end it in an ellipsis when it opens a sheet or a panel that asks for more. Never `OK`, never `Yes`.

Use `hs-btn--primary` for the one default action in a view, and only one: it is the `accent` fill and it is what `⏎` triggers. Use the bare `hs-btn` — white, bordered — for everything else, `hs-btn--plain` for an action that sits inside a row and should not compete with it, and `hs-btn--danger` — `danger` text on a bordered button — for a destructive action in a pane.

`hs-btn--danger-fill` is reserved for the confirm button inside a confirmation sheet, where it is the only filled control on screen. Never put a filled danger button in a toolbar or a list row.

Height follows position, not importance: `hs-btn--lg` in a toolbar and for a sheet's default button, the default `control-height-md` everywhere else, `hs-btn--sm` inside a table row.

Do not disable a button to signal that something is missing. Leave it enabled and explain in a `Banner` what is needed, unless the action is genuinely impossible — then apply `is-disabled` and keep the label unchanged.

Keyboard focus is `is-focused`: a solid `focus-ring` ring drawn 2px outside the button. It is never suppressed, including on a plain button.
