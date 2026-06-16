#!/usr/bin/env bash
# hub-ensure.sh — SessionStart hook: ensure the hub daemon is running.
#
# Probes http://localhost:7777/health and checks for the machine-hub identity.
# If not healthy, starts hub serve in the background and polls up to 5 seconds.
# Always exits 0 — this hook must never abort a session.

set -uo pipefail

HUB_BIN="${CLAUDE_PLUGIN_ROOT}/hub/target/release/hub"
HEALTH_URL="http://localhost:7777/health"

hub_healthy() {
  curl -sf --max-time 2 "$HEALTH_URL" 2>/dev/null | grep -q '"hub":"machine-hub"'
}

if hub_healthy; then
  exit 0
fi

if [ ! -x "$HUB_BIN" ]; then
  echo "hub-ensure: hub binary not found at $HUB_BIN — skipping daemon start" >&2
  exit 0
fi

nohup "$HUB_BIN" serve >/dev/null 2>&1 &

i=0
while [ $i -lt 10 ]; do
  sleep 0.5
  if hub_healthy; then
    exit 0
  fi
  i=$((i + 1))
done

echo "hub-ensure: hub daemon did not become healthy within 5s — session continues without it" >&2
exit 0
