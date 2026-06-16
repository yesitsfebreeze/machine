#!/usr/bin/env bash
# taskboard launcher — self-healing MCP entry point.
#
# Why this exists: Claude Code launches MCP servers with a minimal environment whose
# PATH often omits ~/.local/bin, and in a repo that was never bootstrapped the binary
# may be absent entirely (or a stale, protocol-skewed version) — all of which the
# harness collapses to an opaque `Failed to reconnect: -32000`. mesh avoids this by
# shipping its runtime in-plugin; taskboard is a downloaded binary, so this launcher
# sources the shared installer (install.sh, the single source for the pinned version
# and the prebuilt download), ensures a correct binary is present — installing it on
# demand the way a fresh foreign repo needs — and only then exec's it by absolute path.
# plugin.json invokes this via ${CLAUDE_PLUGIN_ROOT}/taskboard/launch.sh.
#
# All arguments (e.g. `mcp`) are forwarded verbatim to the taskboard binary.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=install.sh
. "$HERE/install.sh"

# Self-heal: install the pinned prebuilt if the binary is missing or version-skewed.
if ! taskboard_ensure; then
  {
    echo "taskboard: no usable binary, and the pinned prebuilt ($TASKBOARD_VERSION) could not be installed."
    echo "  Needs curl + tar + network access, or run scripts/bootstrap.sh on this host, then restart Claude."
  } >&2
  exit 127
fi

bin="$(taskboard_bin_path)" || {
  echo "taskboard: binary not found after install attempt (looked in $TASKBOARD_LOCAL_BIN, /usr/local/bin, /opt/homebrew/bin, PATH)." >&2
  exit 127
}

exec "$bin" "$@"
