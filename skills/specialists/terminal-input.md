# terminal-input

**When**: handling key events, mouse, paste, modifiers; choosing input protocol.
**Why care**: legacy mode loses keys (e.g., Ctrl+Shift+letter), paste-as-keystrokes corrupts data, mouse without enable leaks coordinates.

## Decision tree
- Need Ctrl+Shift / multi-mod combos → Kitty keyboard protocol. Reason: legacy CSI can't represent them.
- Need paste detection → bracketed paste. Reason: avoids interpreting pasted newlines as commands.
- Need mouse → enable explicit mode, disable on exit. Reason: orphaned mode leaks coords into shell.
- Targeting older terminals → degrade gracefully when protocol unsupported. Reason: probe response is optional.

## Tradeoffs
- Kitty protocol: full fidelity. Pay: not universally supported; need fallback.
- Bracketed paste: clean paste handling. Pay: marker stripping logic.
- Raw mode: full control. Pay: must restore on every exit path (panics included).

## Anti-patterns (why)
- Forgetting to disable mouse/raw on panic: leaves user's terminal broken.
- Treating paste chunks as typing: triggers autocomplete, command exec, garbage.
- Polling input on a timer: latency floor; prefer event-driven blocking read.
