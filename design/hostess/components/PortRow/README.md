# PortRow

One taken port. You provide the port, the process, the PID, the owning project when it is known, and the action.

The table lists what is **taken**, never what is free. "Which port is free" is a question nobody asks; "what is on 5173" is asked ten times a day.

The port number is `mono-strong` in a fixed 56px column so the numbers line up. The process name is `text-body`; everything else — PID, path, container name — is `mono-caption` `ink-secondary` on the second line, middle-truncated.

Three ownership states, each with a `StatusDot` and a word: **Yours** (`success`) for a process Hostess or the current project started, **Container** (`idle`) for something inside Docker, **Unknown** (`danger`) for a process Hostess cannot attribute.

The action follows the state. A container gets `Stop`, which stops the container. A process Hostess started gets `Kill`, which is Deliberate. An unknown process gets `Kill` too, but it is Dangerous: a sheet names the process and PID in `mono-body` before anything is sent a signal.

Send `SIGTERM` first and `SIGKILL` only after the person confirms a second time. Report the result as a fact — "Port 5173 released".

Scanning streams into the table as results arrive; it never blocks on a full scan.
