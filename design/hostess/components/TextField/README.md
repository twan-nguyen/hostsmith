# TextField

A single-line input. You provide the value, the placeholder, the validation state and the handler.

Use `hs-field--mono` for anything a developer could paste into a terminal — an IP, a hostname, a port, a variable name or value. A hostname typed into a proportional field is a bug.

Use `hs-field--search` for filtering a list: pill radius, `surface-sunken` ground, a leading magnifier, and filtering as the person types with no Search button.

The placeholder names the thing expected (`Profile name`, `Search entries`), never an instruction and never an example value that looks real enough to be submitted by mistake.

Validate on blur, not on every keystroke, except where the format is unambiguous (an IP address). Mark the failure with `is-invalid` and put the reason in `text-caption` `danger` below the field — never inside it, never as a tooltip.

Focus is a solid `focus-ring`, and the field is `control-height-md` unless it sits in a toolbar, where it is `control-height-lg`.

A field holding a secret is masked by default and reveals only through the `KeyValueRow` reveal flow, never through a plain show/hide button.
