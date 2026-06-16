#!/usr/bin/env bash
# machine — taskboard prebuilt installer (single source of truth)
#
# Sourced by both scripts/bootstrap.sh (session bring-up) and taskboard/launch.sh
# (self-healing MCP launch). Defines the pinned release version, platform detection,
# the prebuilt download/install, binary resolution, and a version-marker file. The
# taskboard binary has no `version` subcommand, so the marker beside it is the only
# way to detect a version-skewed (stale) install. Safe to source: it only defines
# variables and functions — it runs nothing and sets no shell options of its own, so
# it never disturbs the sourcing shell.

# Pinned release — bump deliberately to upgrade (never track latest). THIS is the
# single definition; bootstrap.sh and launch.sh both read it from here.
TASKBOARD_VERSION="v0.6.0"
TASKBOARD_REPO="https://github.com/tcarac/taskboard"

# Managed install location and the version marker written beside the binary. Honour an
# existing LOCAL_BIN (bootstrap sets one) but never require it.
TASKBOARD_LOCAL_BIN="${TASKBOARD_LOCAL_BIN:-${LOCAL_BIN:-$HOME/.local/bin}}"
TASKBOARD_VERSION_MARKER="${TASKBOARD_LOCAL_BIN}/.taskboard.version"

# Map uname to the release asset's os-arch token (echo it), else return non-zero.
taskboard_platform() {
  local os arch
  case "$(uname -s)" in
    Linux)  os="linux" ;;
    Darwin) os="darwin" ;;
    *) return 1 ;;
  esac
  case "$(uname -m)" in
    x86_64|amd64)  arch="amd64" ;;
    arm64|aarch64) arch="arm64" ;;
    *) return 1 ;;
  esac
  echo "${os}-${arch}"
}

# Resolve a usable taskboard binary: the managed location first, then common system
# paths, then PATH. Echo the path and return 0, or return non-zero if none is found.
taskboard_bin_path() {
  local cand
  for cand in "$TASKBOARD_LOCAL_BIN/taskboard" /usr/local/bin/taskboard /opt/homebrew/bin/taskboard; do
    if [ -x "$cand" ]; then echo "$cand"; return 0; fi
  done
  cand="$(command -v taskboard 2>/dev/null || true)"
  if [ -n "$cand" ]; then echo "$cand"; return 0; fi
  return 1
}

# Download + install the pinned prebuilt binary into $TASKBOARD_LOCAL_BIN and record
# the version marker. Returns non-zero on any failure so a caller can fall back to a
# source build. All progress goes to stderr — stdout is reserved for the MCP stream.
taskboard_install_prebuilt() {
  command -v curl >/dev/null 2>&1 || return 1
  command -v tar  >/dev/null 2>&1 || return 1
  local plat; plat="$(taskboard_platform)" || return 1
  local url="$TASKBOARD_REPO/releases/download/$TASKBOARD_VERSION/taskboard-${plat}.tar.gz"
  local tmp; tmp="$(mktemp -d)" || return 1
  echo "  downloading taskboard $TASKBOARD_VERSION ($plat) ..." >&2
  if ! curl -fsSL "$url" -o "$tmp/taskboard.tar.gz" 2>/dev/null; then rm -rf "$tmp"; return 1; fi
  if ! tar -xzf "$tmp/taskboard.tar.gz" -C "$tmp" 2>/dev/null; then rm -rf "$tmp"; return 1; fi
  # The archive may hold the binary as taskboard-<plat> or plain taskboard. Find the
  # first match without a `| head` pipe — bootstrap.sh shadows the coreutils `head`
  # with a function, so a pipe through head there would misbehave.
  local bin="" cand
  while IFS= read -r cand; do bin="$cand"; break; done < <(
    find "$tmp" -type f \( -name 'taskboard' -o -name "taskboard-${plat}" \) 2>/dev/null
  )
  if [ -z "$bin" ]; then rm -rf "$tmp"; return 1; fi
  mkdir -p "$TASKBOARD_LOCAL_BIN"
  if ! install -m 0755 "$bin" "$TASKBOARD_LOCAL_BIN/taskboard" 2>/dev/null; then
    cp "$bin" "$TASKBOARD_LOCAL_BIN/taskboard" && chmod +x "$TASKBOARD_LOCAL_BIN/taskboard" || { rm -rf "$tmp"; return 1; }
  fi
  printf '%s\n' "$TASKBOARD_VERSION" > "$TASKBOARD_VERSION_MARKER" 2>/dev/null || true
  rm -rf "$tmp"
  return 0
}

# Ensure a pinned binary is in place. (Re)install the prebuilt when the binary is
# missing, or when our marker records a version other than the pinned one (skew). A
# present binary with no marker is left as-is — that is an externally managed install
# this machine did not place, and silently overwriting it would be surprising. Returns
# 0 when a usable binary is in place, non-zero if an install was needed and failed.
taskboard_ensure() {
  if ! taskboard_bin_path >/dev/null 2>&1; then
    taskboard_install_prebuilt
    return $?
  fi
  if [ -f "$TASKBOARD_VERSION_MARKER" ] \
     && [ "$(cat "$TASKBOARD_VERSION_MARKER" 2>/dev/null)" != "$TASKBOARD_VERSION" ]; then
    taskboard_install_prebuilt
    return $?
  fi
  return 0
}
