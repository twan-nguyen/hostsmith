# StatusDot

An 8px dot plus the word for the state. You provide the state; the component provides the pair.

The word is not optional. Green and red are the first pair colour-blind developers lose, so a dot on its own is never enough — and the accessibility label is the word, not the colour ("Applied", never "green check").

Four states, and only four:

| Class | Token | Means |
| --- | --- | --- |
| `hs-dot--ok` | `success` | The file on disk matches what Hostess shows. |
| `hs-dot--warn` | `warning` | Something changed outside Hostess, or a variable is missing. |
| `hs-dot--danger` | `danger` | A write failed, or a port is held by an unknown process. |
| `hs-dot--idle` | `control-track` + `border-control` | Nothing is wrong; this thing is simply not active. |

The idle dot keeps its outline. It is the only dot that would otherwise be invisible on `surface-content`, and the outline is what carries it.

Use the past tense and a number where there is one: `Applied 12 entries`, `3 variables missing`. Never `OK` and never `Error`.

In a list row the dot goes on the trailing edge; in a header it goes before the title.
