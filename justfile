# machine — release helpers
# Run `just` with no args to list recipes.

set shell := ["bash", "-uc"]

plugin := ".claude-plugin/plugin.json"
marketplace := ".claude-plugin/marketplace.json"

# Default: show available recipes.
default:
    @just --list

# Install and verify every machine dependency (kern, hub, git-fs, MCP servers).
# Idempotent — skips what is already present. Safe to re-run.
bootstrap:
    @bash scripts/bootstrap.sh

# Remove regenerable build artifacts from the tree. Safe — touches nothing
# tracked and leaves daemon state (.kern/.mesh/.board/.git-fs) intact.
# Re-create with `just bootstrap` / `just graphify`.
clean:
    #!/usr/bin/env bash
    set -uo pipefail
    rm -rf node_modules hub/target .machine/worktrees
    rm -f .machine/graph.json .machine/board.json .machine/ENV.md version.log
    echo "cleaned: build artifacts removed (daemon state preserved)"

# Rebuild the repo capability graph (.machine/graph.json) that the default agent
# reads. Runs automatically on every push via the .git/hooks/pre-push hook
# (installed by `just bootstrap`); run manually here anytime.
graphify:
    @node scripts/graphify.mjs

# Dev the hub board UI: build once, then serve with HUB_DEV=1 so board_ui.html
# is re-read from disk on every request — edit the HTML and just refresh the
# browser, no rebuild. Stop with Ctrl-C. For .rs changes, see `just hub-watch`.
hub-dev:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build --release --manifest-path hub/Cargo.toml
    pkill -xf './hub/target/release/hub serve' 2>/dev/null || true
    sleep 1
    echo "serving http://localhost:7777 (HUB_DEV=1 — live UI reload)"
    HUB_DEV=1 ./hub/target/release/hub serve

# Auto rebuild+restart the hub on any Rust change (needs `cargo install bacon`
# or cargo-watch). Pairs with HUB_DEV=1 for the UI. Uses cargo-watch if present.
hub-watch:
    #!/usr/bin/env bash
    set -euo pipefail
    if command -v bacon >/dev/null; then
        exec bacon --job run -- --manifest-path hub/Cargo.toml -- serve
    elif command -v cargo-watch >/dev/null; then
        exec cargo watch -w hub/src -x 'run --release --manifest-path hub/Cargo.toml -- serve'
    else
        echo "install one: cargo install bacon   (or)   cargo install cargo-watch" >&2
        exit 1
    fi

# Bump every version field by 0.0.1 (patch), with odometer turnover at 10
# (x.x.9 -> x.(x+1).0, x.9.9 -> (x+1).0.0). Prints "OLD -> NEW".
bump:
    #!/usr/bin/env bash
    set -euo pipefail
    cur=$(grep -m1 '"version"' "{{plugin}}" | sed -E 's/.*"version"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/')
    IFS=. read -r major minor patch <<< "$cur"
    patch=$((patch + 1))
    if (( patch >= 10 )); then patch=0; minor=$((minor + 1)); fi
    if (( minor >= 10 )); then minor=0; major=$((major + 1)); fi
    new="${major}.${minor}.${patch}"
    # Replace the exact current version string in both manifests (keeps the
    # plugin + marketplace entries in lockstep).
    sed -i "s/\"version\"\([[:space:]]*\):\([[:space:]]*\)\"${cur}\"/\"version\"\1:\2\"${new}\"/g" "{{plugin}}" "{{marketplace}}"
    node -e 'JSON.parse(require("fs").readFileSync("{{plugin}}"));JSON.parse(require("fs").readFileSync("{{marketplace}}"))'
    echo "${cur} -> ${new}"

# Bump versions, commit everything, and push to the current branch.
push message="":
    #!/usr/bin/env bash
    set -euo pipefail
    line=$(just bump)
    new="${line##*-> }"
    msg="${message:-chore(release): v${new}}"
    git add -A
    git commit -q -m "$msg"
    git push origin "$(git rev-parse --abbrev-ref HEAD)"
    echo "pushed v${new}"
