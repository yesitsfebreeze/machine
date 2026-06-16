#!/usr/bin/env bash
# machine — taskboard project resolver
#
# Resolve-or-create the taskboard project for the current working directory and
# persist its identity to .machine/taskboard.json. One taskboard project = one cwd
# (board-per-cwd). Run from a repo root; pass an explicit dir as $1 to override.
#
# Keying (deterministic, agent-independent):
#   cwd    = absolute working directory
#   name   = basename(cwd)
#   prefix = "P" + first 6 UPPER hex chars of sha1(absolute cwd)
# The prefix is stable and collision-resistant; the absolute cwd is persisted in
# taskboard.json so a prefix collision can be detected locally. The richer remote
# disambiguator (the absolute cwd written into the project *description*, §8.2 of
# the plan) is set by the drill agent through the taskboard MCP create_project /
# update_project tools, which accept a description field — the `taskboard project
# create` CLI used here does not. This helper is the deterministic fallback path.
#
# Card-face metadata: this helper sets --icon and --color at create time (the CLI
# supports both). Override per repo with TASKBOARD_ICON / TASKBOARD_COLOR. taskboard
# has no project-update CLI, so icon and color apply only on creation — an existing
# project keeps whatever it was created with.
#
# Degrades cleanly: if the `taskboard` binary is absent it prints a clear message
# and exits non-zero without touching taskboard.json.

set -uo pipefail

CWD="${1:-$PWD}"
CWD="$(cd "$CWD" 2>/dev/null && pwd)" || { echo "taskboard-resolve: cannot resolve dir '$1'" >&2; exit 1; }

if ! command -v taskboard >/dev/null 2>&1; then
  echo "taskboard-resolve: taskboard binary not found — run scripts/bootstrap.sh to install it (board projection stays off until then)" >&2
  exit 1
fi

NAME="$(basename "$CWD")"
HASH="$(printf '%s' "$CWD" | sha1sum | cut -c1-6 | tr '[:lower:]' '[:upper:]')"
PREFIX="P${HASH}"
URL="http://localhost:3010"
OUT=".machine/taskboard.json"
ICON="${TASKBOARD_ICON:-⚙️}"
COLOR="${TASKBOARD_COLOR:-#4F46E5}"

# `project list` prints one line per project, the id last after " - ":
#   <name> [<prefix>] (<status>) - <id>
# Match our prefix in literal brackets, then take the id after the final " - ".
existing_id=""
while IFS= read -r line; do
  case "$line" in
    *"[$PREFIX]"*)
      existing_id="$(printf '%s' "$line" | sed -n 's/.* - \([^ ]*\)[[:space:]]*$/\1/p')"
      break
      ;;
  esac
done < <(taskboard project list 2>/dev/null)

if [ -n "$existing_id" ]; then
  project_id="$existing_id"
else
  created="$(taskboard project create "$NAME" --prefix "$PREFIX" --icon "$ICON" --color "$COLOR" 2>&1)" || {
    echo "taskboard-resolve: project create failed: $created" >&2
    exit 1
  }
  # `project create` prints: "Created project <name> [<prefix>] (<id>)"
  project_id="$(printf '%s' "$created" | sed -n 's/.*(\([^()]*\))[[:space:]]*$/\1/p')"
  if [ -z "$project_id" ]; then
    echo "taskboard-resolve: could not parse new project id from: $created" >&2
    exit 1
  fi
fi

resolved_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
mkdir -p "$(dirname "$OUT")"
cat > "$OUT" <<JSON
{
  "version": 1,
  "cwd": "$CWD",
  "name": "$NAME",
  "prefix": "$PREFIX",
  "projectId": "$project_id",
  "icon": "$ICON",
  "color": "$COLOR",
  "url": "$URL",
  "resolvedAt": "$resolved_at"
}
JSON

echo "taskboard-resolve: project $NAME [$PREFIX] -> $project_id (written to $OUT)"
