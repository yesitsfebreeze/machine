# ansi-pty

**When**: emitting escape sequences, parsing terminal output, deciding capability detection strategy.
**Why care**: wrong sequence garbles the terminal; capability assumptions break on minority terminals.

## Decision tree
- Targeting modern terminals → assume truecolor + UTF-8. Reason: covers >95% of dev environments.
- Need wide portability → query terminfo / `tput`. Reason: source of truth for capabilities.
- Inside a PTY → handle resize signals. Reason: terminal size changes mid-session and consumers need it.
- Rendering controlled by a TUI lib → let the lib own escapes. Reason: mixing layers corrupts state.

## Tradeoffs
- Hardcoded escapes: simple, fast. Pay: portability.
- terminfo: portable. Pay: indirection, learning curve.
- Probing (e.g., DA1/DA2): accurate. Pay: latency, fragile parsing.

## Anti-patterns (why)
- Printing escapes to non-TTY: garbles logs and pipes.
- Assuming xterm-256color everywhere: legacy/CI terminals will show literal escapes.
- Ignoring SIGWINCH: stale dimensions cause wrap and misalignment.
