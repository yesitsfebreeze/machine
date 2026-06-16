#!/usr/bin/env bash
# machine — dependency bootstrap
#
# Installs and verifies everything the machine depends on, in one pass:
#   kern         memory daemon          (release installer, cargo fallback)
#   hub          coordination daemon    (cargo build --release --manifest-path hub/Cargo.toml)
#   git-fs       companion plugin       (claude plugin install)
#   context-mode vendored MCP server    (npx on demand; needs Node >=22.5.0)
#   context7     vendored MCP server    (needs CONTEXT7_API_KEY)
#   pdf-reader   vendored MCP server    (npx on demand)
#   board        addon: kanban MCP+web  (ships in-plugin; needs only Node)
#
# board is an optional addon, not a core daemon: it ships in-plugin (a single
# zero-dep Node file), so there is nothing to install — bring-up only starts its
# web daemon. If Node is missing or the daemon fails to bind it warns and skips
# (the web view is off, MCP card ops still work) — it never aborts the run.
#
# Idempotent: already-satisfied dependencies are skipped. Safe to re-run.
# Usage: scripts/bootstrap.sh   (or: just bootstrap)

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
KERN_INSTALLER="https://raw.githubusercontent.com/yesitsfebreeze/relay-kern/master/install.sh"

# --- output helpers ---------------------------------------------------------
bold=$(tput bold 2>/dev/null || true); dim=$(tput dim 2>/dev/null || true)
red=$(tput setaf 1 2>/dev/null || true); grn=$(tput setaf 2 2>/dev/null || true)
ylw=$(tput setaf 3 2>/dev/null || true); rst=$(tput sgr0 2>/dev/null || true)

declare -a SUMMARY
ok()   { echo "  ${grn}ok${rst}    $1";   SUMMARY+=("ok|$1"); }
did()  { echo "  ${grn}done${rst}  $1";   SUMMARY+=("done|$1"); }
warn() { echo "  ${ylw}warn${rst}  $1";   SUMMARY+=("warn|$1"); }
fail() { echo "  ${red}fail${rst}  $1";   SUMMARY+=("fail|$1"); }
head() { echo; echo "${bold}$1${rst}"; }

have() { command -v "$1" >/dev/null 2>&1; }

# Node >= 22.5.0 ?
node_ok() {
  have node || return 1
  local v major minor
  v="$(node -v 2>/dev/null | sed 's/^v//')"
  major="${v%%.*}"; minor="$(echo "$v" | cut -d. -f2)"
  [ "${major:-0}" -gt 22 ] && return 0
  [ "${major:-0}" -eq 22 ] && [ "${minor:-0}" -ge 5 ] && return 0
  return 1
}

# ~/.local/bin — user-owned; must be on PATH for the links/shim below to win.
LOCAL_BIN="${HOME}/.local/bin"
on_path() { case ":$PATH:" in *":$1:"*) return 0;; *) return 1;; esac; }

# Echo a node binary's version if it is >= 22.5.0, else return non-zero.
node_bin_ok() {
  local bin="$1" v major minor
  [ -x "$bin" ] || return 1
  v="$("$bin" -v 2>/dev/null | sed 's/^v//')" || return 1
  major="${v%%.*}"; minor="$(echo "$v" | cut -d. -f2)"
  { [ "${major:-0}" -gt 22 ]; } || { [ "${major:-0}" -eq 22 ] && [ "${minor:-0}" -ge 5 ]; } || return 1
  echo "$v"
}

# Find the newest Node >=22.5.0 already installed under nvm/fnm. Echo its bin dir.
find_modern_node() {
  local best="" bestv="" cand v
  for cand in \
    "$HOME"/.nvm/versions/node/v*/bin/node \
    "$HOME"/.local/share/fnm/node-versions/*/installation/bin/node \
    "$HOME"/.fnm/node-versions/*/installation/bin/node; do
    [ -x "$cand" ] || continue
    v="$(node_bin_ok "$cand")" || continue
    if [ -z "$bestv" ] || [ "$(printf '%s\n%s\n' "$bestv" "$v" | sort -V | tail -1)" = "$v" ]; then
      bestv="$v"; best="$(dirname "$cand")"
    fi
  done
  [ -n "$best" ] && echo "$best"
}

# Ensure the harness resolves Node >=22.5.0. If system Node is too old but a modern
# Node exists under nvm/fnm, link node/npm/npx into ~/.local/bin (idempotent).
ensure_node() {
  head "Node (>=22.5.0 for context-mode / pdf-reader)"
  if node_ok; then ok "Node $(node -v) satisfies >=22.5.0"; return; fi
  local bindir; bindir="$(find_modern_node)"
  if [ -z "$bindir" ]; then
    warn "Node $(node -v 2>/dev/null || echo 'missing') < 22.5.0 and no newer Node found — install one (e.g. nvm install 22)"; return
  fi
  if ! on_path "$LOCAL_BIN"; then
    warn "modern Node at $bindir but $LOCAL_BIN not on PATH — add it to PATH, then re-run"; return
  fi
  mkdir -p "$LOCAL_BIN"
  ln -sf "$bindir/node" "$LOCAL_BIN/node"
  ln -sf "$bindir/npm"  "$LOCAL_BIN/npm"
  ln -sf "$bindir/npx"  "$LOCAL_BIN/npx"
  did "linked Node $("$bindir/node" -v) into $LOCAL_BIN (restart Claude to apply)"
}

# Create the git-fs-mcp shim the git-fs plugin expects. The plugin is git-cloned
# (no npm bin link), so its declared `git-fs-mcp` binary never lands on PATH; point
# a shim at the newest cached plugin server. Idempotent.
ensure_gitfs_shim() {
  head "git-fs-mcp (MCP launcher shim)"
  if have git-fs-mcp && [ ! -L "$LOCAL_BIN/git-fs-mcp" ]; then
    ok "git-fs-mcp already on PATH ($(command -v git-fs-mcp))"; return
  fi
  local base="$HOME/.claude/plugins/cache/git-fs/git-fs"
  if [ ! -d "$base" ]; then
    warn "git-fs plugin not installed yet — re-run after /plugin install git-fs@git-fs to create the shim"; return
  fi
  if ! on_path "$LOCAL_BIN"; then
    warn "$LOCAL_BIN not on PATH — cannot place git-fs-mcp shim; add it and re-run"; return
  fi
  mkdir -p "$LOCAL_BIN"
  cat > "$LOCAL_BIN/git-fs-mcp" <<'SHIM'
#!/usr/bin/env bash
# Auto-generated by machine bootstrap. Launches the git-fs plugin's MCP server
# (newest cached version with dist/mcp.js). Safe to delete; bootstrap recreates it.
set -euo pipefail
base="$HOME/.claude/plugins/cache/git-fs/git-fs"
ver="$(ls -1 "$base" 2>/dev/null | sort -V | tac | while read -r v; do [ -f "$base/$v/dist/mcp.js" ] && { echo "$v"; break; }; done)"
[ -n "${ver:-}" ] || { echo "git-fs-mcp: no dist/mcp.js under $base" >&2; exit 127; }
exec node "$base/$ver/dist/mcp.js" "$@"
SHIM
  chmod +x "$LOCAL_BIN/git-fs-mcp"
  did "git-fs-mcp shim written to $LOCAL_BIN (restart Claude to apply)"
}

# Install/refresh the status-line shim at a STABLE, version-independent path.
# A plugin cannot contribute a main `statusLine` (Claude Code reads it only from
# user/project/local settings), and the plugin cache dir is versioned — so a repo
# that pointed at `.../machine/<version>/.claude/hooks/statusline.mjs` would rot on
# every `/plugin update`. Copy the script to ~/.claude/hooks/machine-statusline.mjs
# instead; repos point at that one stable path and never need re-wiring. Idempotent:
# overwrites on every run so a plugin update ships the latest script automatically.
ensure_statusline_shim() {
  head "statusline (stable shim)"
  local src="$REPO_ROOT/.claude/hooks/statusline.mjs"
  local dst="$HOME/.claude/hooks/machine-statusline.mjs"
  if [ ! -f "$src" ]; then
    warn "statusline source not found at $src — cannot install shim"; return
  fi
  mkdir -p "$(dirname "$dst")"
  if cp "$src" "$dst"; then
    did "statusline shim refreshed at $dst (repos point at it via ~/.claude/hooks/machine-statusline.mjs)"
  else
    warn "could not write statusline shim to $dst"
  fi
}

# --- kern -------------------------------------------------------------------
install_kern() {
  head "kern (memory daemon)"
  if have kern; then ok "kern present ($(command -v kern))"; return; fi
  if have curl; then
    echo "  installing via release installer..."
    if curl -fsSL "$KERN_INSTALLER" | sh; then did "kern installed"; return; fi
    warn "release installer failed"
  fi
  if have cargo; then
    echo "  falling back to cargo..."
    if cargo install --git https://github.com/yesitsfebreeze/relay-kern kern; then
      did "kern installed (cargo)"; return
    fi
  fi
  fail "kern not installed — see https://github.com/yesitsfebreeze/kern"
}

# --- hub --------------------------------------------------------------------
install_hub() {
  head "hub (coordination daemon)"
  if [ -x "$REPO_ROOT/hub/target/release/hub" ]; then
    ok "hub present ($REPO_ROOT/hub/target/release/hub)"; return
  fi
  if [ ! -d "$REPO_ROOT/hub" ]; then
    fail "hub source not found at $REPO_ROOT/hub — clone the machine repo to build it"; return
  fi
  if ! have cargo; then
    fail "cargo (Rust toolchain) required to build hub — https://rustup.rs"; return
  fi
  echo "  building from $REPO_ROOT/hub..."
  if cargo build --release --manifest-path "$REPO_ROOT/hub/Cargo.toml"; then did "hub built"; else fail "hub build failed"; fi
}

# --- git-fs plugin ----------------------------------------------------------
install_gitfs() {
  head "git-fs (companion plugin)"
  if ! have claude; then warn "claude CLI not found — install git-fs manually: /plugin install git-fs@git-fs"; return; fi
  # The claude CLI cannot be nested inside a running Claude Code session
  # (sandbox isolation). Defer to the in-session /plugin commands instead.
  if [ -n "${CLAUDECODE:-}" ]; then
    warn "in a Claude Code session — run in-session: /plugin marketplace add yesitsfebreeze/git-fs then /plugin install git-fs@git-fs"
    return
  fi
  if claude plugin list 2>/dev/null | grep -qi "git-fs"; then ok "git-fs plugin already installed"; return; fi
  echo "  adding marketplace + installing..."
  claude plugin marketplace add yesitsfebreeze/git-fs >/dev/null 2>&1 || true
  if claude plugin install git-fs@git-fs >/dev/null 2>&1; then
    did "git-fs installed (run /reload-plugins to apply)"
  else
    warn "git-fs install failed — try: claude plugin install git-fs@git-fs"
  fi
}

# --- context7 + pdf-reader (vendored) --------------------------------------
check_others() {
  head "context7 + pdf-reader (vendored MCP)"
  if [ -n "${CONTEXT7_API_KEY:-}" ]; then ok "CONTEXT7_API_KEY is set"; else warn "CONTEXT7_API_KEY unset — context7 docs server will be unauthenticated"; fi
  if have npx; then ok "npx present — pdf-reader fetches on demand"; else warn "npx not found — pdf-reader needs Node.js"; fi
}

# --- graphify pre-push hook (repo-local; never ships) ----------------------
install_graphify() {
  head "graphify pre-push hook"
  local hook=".git/hooks/pre-push"
  if [ ! -d .git ]; then warn "no .git dir — skipping graphify hook"; return; fi
  cat > "$hook" <<'HOOK'
#!/usr/bin/env bash
# machine: graphify — rebuild the repo capability graph on every push.
# Repo-local, never travels (lives in .git/, not the plugin payload).
# NEVER blocks the push: any failure is a warning, exit is always 0.
root="$(git rev-parse --show-toplevel 2>/dev/null)" || exit 0
if command -v node >/dev/null 2>&1; then
  node "$root/scripts/graphify.mjs" || echo "graphify: skipped (non-fatal)" >&2
fi
exit 0
HOOK
  chmod +x "$hook"
  if have node; then node scripts/graphify.mjs >/dev/null 2>&1 && ok "graphify hook installed + initial graph built (.machine/graph.json)" || ok "graphify hook installed (initial build deferred)"; else warn "node not found — graphify hook installed but cannot run until node is present"; fi
}

# --- board (addon: local kanban MCP + web) ---------------------------------
# board ships in-plugin as a single zero-dependency Node file (board/board.mjs),
# so there is nothing to install — bring-up only starts its web daemon. The MCP
# surface is launched on demand by the harness (plugin.json), independent of this
# daemon. Rollback: remove the plugin.json `board` mcpServers entry and
# mine/skills/board/, delete board/, optionally `node board/board.mjs stop`.
# Note: the `board` mcpServers entry only registers its MCP tools after a Claude
# Code restart — sessions started before it was added must restart to get them.
BOARD_PORT="3010"

# Board-specific liveness probe (bounded). The backgrounded `serve` binds
# asynchronously, so confirm readiness before declaring it up. We probe
# /healthz and require the board's identity signature — NOT a bare 200 on
# /api/board, which a foreign daemon (e.g. a stale taskboard) squatting the same
# port also answers, which would make us mistake the squatter for the board.
board_ready() {
  local i
  for i in 1 2 3 4 5 6 7 8 9 10; do
    if board_signature_present; then
      return 0
    fi
    sleep 1
  done
  return 1
}

# True iff :$BOARD_PORT is served by a real board.mjs (its /healthz identity).
board_signature_present() {
  have curl || return 1
  curl -fsS "http://localhost:${BOARD_PORT}/healthz" 2>/dev/null | grep -q '"board":"machine-board"'
}

# PID of whatever is listening on :$BOARD_PORT, or empty if the port is free.
board_port_pid() {
  if have lsof; then
    lsof -tiTCP:"$BOARD_PORT" -sTCP:LISTEN 2>/dev/null | head -1
  elif have fuser; then
    fuser "${BOARD_PORT}/tcp" 2>/dev/null | tr -d ' ' | head -1
  fi
}

start_board() {
  head "board (addon: kanban MCP + web board)"
  if ! node_ok; then
    warn "Node >=22.5.0 not resolvable — board web view off (MCP card ops still work)"
    return
  fi
  # Already up? Require the board's own /healthz signature, not any listener.
  if board_signature_present; then
    ok "board daemon already running (http://localhost:$BOARD_PORT)"
    return
  fi
  # A foreign listener squatting the port (e.g. a stale taskboard daemon that
  # survived an addon removal) is NOT the board. Clear the known culprit via its
  # own CLI, else fail loudly — never silently treat it as the board.
  local squatter; squatter="$(board_port_pid)"
  if [ -n "$squatter" ]; then
    warn "port :$BOARD_PORT held by non-board listener (pid $squatter) — board.mjs is not serving here"
    if have taskboard; then
      echo "  stopping stale taskboard daemon ..."
      taskboard stop >/dev/null 2>&1 || true
      sleep 1
    fi
    squatter="$(board_port_pid)"
    if [ -n "$squatter" ]; then
      warn "could not free :$BOARD_PORT (pid $squatter still listening) — stop it manually, then re-run bootstrap"
      return
    fi
  fi
  echo "  starting board web daemon on :$BOARD_PORT ..."
  ( cd "$REPO_ROOT" && nohup node board/board.mjs serve >/dev/null 2>&1 & )
  if board_ready; then
    did "board daemon up (http://localhost:$BOARD_PORT)"
  else
    warn "board daemon did not become ready on :$BOARD_PORT — MCP card ops still work without the web UI"
  fi
}

# --- run --------------------------------------------------------------------
echo "${bold}machine bootstrap${rst} ${dim}(idempotent — re-run anytime)${rst}"
install_kern
install_hub
install_gitfs
ensure_gitfs_shim
ensure_statusline_shim
ensure_node
check_others
start_board
install_graphify

head "summary"
fails=0; warns=0
for row in "${SUMMARY[@]}"; do
  status="${row%%|*}"; msg="${row#*|}"
  case "$status" in
    fail) echo "  ${red}x${rst} $msg"; fails=$((fails+1));;
    warn) echo "  ${ylw}!${rst} $msg"; warns=$((warns+1));;
    *)    echo "  ${grn}+${rst} $msg";;
  esac
done
echo
if [ "$fails" -gt 0 ]; then
  echo "${red}${bold}$fails dependency/dependencies missing.${rst} Resolve the items above, then re-run."
  exit 1
fi
[ "$warns" -gt 0 ] && echo "${ylw}Bootstrap complete with $warns warning(s).${rst}" || echo "${grn}${bold}All dependencies satisfied.${rst}"
echo "${dim}Restart Claude Code (or /reload-plugins) so MCP servers and plugins load.${rst}"
exit 0
