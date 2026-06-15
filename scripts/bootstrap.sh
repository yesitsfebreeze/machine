#!/usr/bin/env bash
# machine — dependency bootstrap
#
# Installs and verifies everything the machine depends on, in one pass:
#   kern         memory daemon          (release installer, cargo fallback)
#   mesh         coordination daemon    (cargo install --path mesh)
#   git-fs       companion plugin       (claude plugin install)
#   context-mode vendored MCP server    (npx on demand; needs Node >=22.5.0)
#   context7     vendored MCP server    (needs CONTEXT7_API_KEY)
#   pdf-reader   vendored MCP server    (npx on demand)
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

# --- mesh -------------------------------------------------------------------
install_mesh() {
  head "mesh (coordination daemon)"
  if have mesh; then ok "mesh present ($(command -v mesh))"; return; fi
  if [ ! -d "$REPO_ROOT/mesh" ]; then
    fail "mesh source not found at $REPO_ROOT/mesh — clone the machine repo to build it"; return
  fi
  if ! have cargo; then
    fail "cargo (Rust toolchain) required to build mesh — https://rustup.rs"; return
  fi
  echo "  building from $REPO_ROOT/mesh..."
  if cargo install --path "$REPO_ROOT/mesh"; then did "mesh installed"; else fail "mesh build failed"; fi
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

# --- context-mode (vendored, npx) ------------------------------------------
check_context_mode() {
  head "context-mode (vendored MCP — npx)"
  if ! have npx; then fail "npx not found — install Node.js (>=22.5.0)"; return; fi
  if node_ok; then
    ok "Node $(node -v) satisfies >=22.5.0"
  else
    warn "Node $(node -v 2>/dev/null || echo '?') is < 22.5.0 — context-mode will not start. Use a newer Node (e.g. nvm install 22)."
  fi
}

# --- context7 + pdf-reader (vendored) --------------------------------------
check_others() {
  head "context7 + pdf-reader (vendored MCP)"
  if [ -n "${CONTEXT7_API_KEY:-}" ]; then ok "CONTEXT7_API_KEY is set"; else warn "CONTEXT7_API_KEY unset — context7 docs server will be unauthenticated"; fi
  if have npx; then ok "npx present — pdf-reader fetches on demand"; else warn "npx not found — pdf-reader needs Node.js"; fi
}

# --- run --------------------------------------------------------------------
echo "${bold}machine bootstrap${rst} ${dim}(idempotent — re-run anytime)${rst}"
install_kern
install_mesh
install_gitfs
check_context_mode
check_others

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
