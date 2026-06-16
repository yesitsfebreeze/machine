# taskboard MCP server fails to reconnect (-32000) in a foreign repo on v0.4.9

- **date:** 2026-06-16
- **severity:** major
- **area:** mcpServer/taskboard (launch.sh + bootstrap install) + plugin v0.4.9
- **status:** open

## What happened
In a repo *other than* the machine source, on machine plugin **v0.4.9**, Claude
Code's `/mcp` panel reported:

```
Failed to reconnect to plugin:machine:taskboard: -32000
```

The other five plugin MCP servers (kern, mesh, context7, pdf-reader, context-mode)
were unaffected. `-32000` is a generic JSON-RPC server error surfaced on
reconnect — the server process either never came up or exited before the
handshake completed, so the harness could not re-establish the session.

This is the second taskboard-area incident on 2026-06-16 (see the concurrent-driver
and merge-race reports) but a distinct failure mode: those were coordination/merge
bugs; this is a **server-start / reconnect** failure in a consuming repo.

## Expected
`plugin:machine:taskboard` should start and reconnect cleanly in any repo where
the machine plugin is installed, exactly like mesh — or fail loudly with an
actionable message, not an opaque `-32000`.

## Likely cause (unconfirmed — needs the fix pass to verify)
`taskboard/launch.sh` resolves a **downloaded binary** (`~/.local/bin/taskboard`,
then `/usr/local/bin`, `/opt/homebrew/bin`, then PATH). Unlike `mesh` (a committed
`mesh.mjs` shipped *inside* the plugin and run via node), taskboard depends on a
host-side install performed by `scripts/bootstrap.sh`. In a foreign repo that was
never bootstrapped — or bootstrapped against a different taskboard version than the
plugin's v0.4.9 protocol expects — the binary is missing, stale, or
protocol-incompatible:

- **Missing binary:** launch.sh exits 127 with a clear stderr message, but the
  harness may still surface only `-32000` on the reconnect attempt.
- **Version skew:** an older `~/.local/bin/taskboard` binary speaks a different MCP
  protocol than plugin v0.4.9, so the handshake fails after the process starts.
- **PATH/env:** the minimal MCP launch environment (the very reason launch.sh
  exists) still differs enough that the binary aborts at startup.

## Evidence
- User report: `Failed to reconnect to plugin:machine:taskboard: -32000`, machine
  v0.4.9, a repo other than the machine source.
- `taskboard/launch.sh` resolves an external binary by absolute path (see header
  comment); mesh by contrast ships its runtime in-plugin.
- Reproduced symptom class: taskboard is the only plugin MCP server that is NOT
  self-contained in the plugin payload.

## Suggested direction (for the fix pass, not done here)
1. Make taskboard self-contained like mesh (ship the runtime in-plugin) OR pin and
   verify the binary version in launch.sh, failing with an explicit
   "run scripts/bootstrap.sh / version mismatch" message.
2. Have launch.sh emit a version-check line so a protocol skew is visible instead
   of collapsing to `-32000`.
3. Document in bring-up that consuming repos must run bootstrap before taskboard
   works, and surface taskboard's absence as a non-fatal, named condition.
