#!/usr/bin/env bash
# taskboard launcher — mirrors mesh's plugin-dir-prefixed entry point.
#
# Why this exists: Claude Code launches MCP servers with a minimal environment
# whose PATH often omits ~/.local/bin, so a bare `taskboard` command fails to
# resolve and the server never starts. mesh sidesteps this by referencing an
# absolute ${CLAUDE_PLUGIN_ROOT}/mesh/mesh.mjs path; taskboard is a downloaded
# binary (installed by scripts/bootstrap.sh into ~/.local/bin), so this committed
# launcher ships inside the plugin and locates the binary by absolute path before
# exec-ing it. plugin.json invokes this via ${CLAUDE_PLUGIN_ROOT}/taskboard/launch.sh.
#
# All arguments (e.g. `mcp`) are forwarded verbatim to the taskboard binary.
set -euo pipefail

# Resolve the taskboard binary by absolute path, not PATH lookup.
candidates=(
  "${HOME}/.local/bin/taskboard"
  "/usr/local/bin/taskboard"
  "/opt/homebrew/bin/taskboard"
)

bin=""
for cand in "${candidates[@]}"; do
  if [ -x "$cand" ]; then bin="$cand"; break; fi
done

# Last resort: a PATH lookup, in case the install landed somewhere bespoke.
if [ -z "$bin" ]; then
  bin="$(command -v taskboard 2>/dev/null || true)"
fi

if [ -z "$bin" ]; then
  echo "taskboard binary not found (looked in ~/.local/bin, /usr/local/bin, /opt/homebrew/bin, PATH)." >&2
  echo "Install it with scripts/bootstrap.sh, then restart Claude." >&2
  exit 127
fi

exec "$bin" "$@"
