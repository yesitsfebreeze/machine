> Reference for the `drill` skill's bring-up — the one-shot bootstrap drill runs when a repo is not yet set up. Not a registered skill; drill consults it.

# /assemble — bootstrap the machine

The machine ships as a Claude Code **plugin** named `machine`. Installing the
plugin gives you its agents, skills, hooks, and `plugin.json` `mcpServers`, but the plugin alone
cannot make a repo fully operational: the runtime daemons, the companion plugin,
and the per-repo configuration still have to be installed and wired. `/assemble`
is the single entry point that does all of that in one idempotent pass, then hands
off to `/oil` to specialize the project layer.

Division of labour:

- **`/assemble`** — install + configure everything (this skill).
- **`/oil`** — write `/.machine/`, the per-repo project layer (invoked at the end here).
- **`just bootstrap`** — the same dependency install, run from a terminal *outside*
  a Claude Code session. `/assemble` reuses its script (`scripts/bootstrap.sh`);
  there is one source of truth for what gets installed.

## What assemble brings up

| Component | Kind | How |
|---|---|---|
| `kern` | memory daemon | release installer / cargo (via `bootstrap.sh`) |
| `hub` | coordination daemon | bundled Rust binary `hub/target/release/hub` (`cargo build --release`) |
| `git-fs` | companion plugin | `/plugin install git-fs@git-fs` |
| `context-mode` | vendored MCP (`ctx_*`) | runs via `npx`; needs Node >=22.5.0 |
| `context7` | vendored MCP | needs `CONTEXT7_API_KEY` |
| `pdf-reader` | vendored MCP | runs via `npx` on demand |
| `board` | addon: kanban MCP + web board | ships in-plugin as a single zero-dep Node file; `bootstrap.sh` only starts its web daemon on `:3010`; skips with a warning if Node is missing or the daemon fails to bind (MCP card ops still work) |
| `codex-peer-review` | optional addon | copied into project `.claude/skills/` on opt-in (needs OpenAI Codex CLI) |
| status line | per-repo config | wired into project `.claude/settings.json` |
| required keys | per-repo config | recorded in gitignored `settings.local.json` |

## 1. Resolve the machine root

The dependency installer and the `hub` binary ship inside the plugin payload.
Resolve the root once; `${CLAUDE_PLUGIN_ROOT}` expands in skill content to the
installed plugin directory, and falls back to the repo root when developing the
machine itself.

```bash
ROOT="${CLAUDE_PLUGIN_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
echo "machine root: $ROOT"
```

## 2. Install runtime dependencies

Run the bootstrap script. It is idempotent, detects and skips what is already
present, verifies the MCP prerequisites (Node version for context-mode,
`CONTEXT7_API_KEY` for context7, `npx` for pdf-reader), and prints a per-component
summary.

```bash
bash "$ROOT/scripts/bootstrap.sh"
```

Read the summary it prints. Two items the script cannot complete from inside a
running session it will flag as warnings — handle them in steps 3 (git-fs) and 4
(keys). Everything else (kern, hub, Node/npx checks) it resolves directly.

## 3. Install the companion plugin (git-fs)

`git-fs` is a live plugin that ships no standalone binary, so it is installed
through the plugin system, not vendored in the machine's `mcpServers`. The `claude` CLI cannot be
nested inside a running Claude Code session, so `/assemble` cannot install it for
the user from within a session. Detect and guide:

- If `git-fs` already appears in the installed plugins, say so and move on.
- Otherwise present these two commands for the user to run, then `/reload-plugins`:

```
/plugin marketplace add yesitsfebreeze/git-fs
/plugin install git-fs@git-fs
```

When `/assemble`'s work is driven from a terminal instead (via `just bootstrap`),
the script installs git-fs directly through the `claude plugin` CLI; no manual step
is needed there.

## 3b. Optional addon — cross-model peer reviewer (codex-peer-review)

`codex-peer-review` is an **opt-in** addon, not a machine dependency. It shells out
to OpenAI's **Codex CLI** for a second opinion from a different model family on
high-stakes architecture, security, and design decisions, then synthesizes both
views. It is complementary to `/personas` (an intra-Claude panel), not a duplicate.
It is gated because its prerequisite is heavy and external: the OpenAI Codex CLI
plus OpenAI authentication.

The skill ships vendored in the plugin payload at `$ROOT/mine/skills/codex-peer-review`
(`$ROOT` is the machine root resolved in step 1). Activating it copies that
folder into the **target project's** `.claude/skills/`, where Claude Code
auto-discovers it. `/assemble` never edits the machine plugin's own `skills/`.

- If `<project>/.claude/skills/codex-peer-review` already exists, it is active —
  say so and skip the copy.
- Otherwise ask the user once whether to enable the cross-model reviewer. If they
  decline, skip this step entirely.
- On opt-in, copy the vendored folder into the project, then check the Codex CLI:

```bash
SRC="$ROOT/mine/skills/codex-peer-review"   # $ROOT from step 1
DST="$(git rev-parse --show-toplevel)/.claude/skills/codex-peer-review"
if [ -d "$DST" ]; then
  echo "codex-peer-review already active at $DST"
elif [ ! -d "$SRC" ]; then
  echo "vendored skill not found at $SRC — is the machine plugin installed?"
else
  mkdir -p "$(dirname "$DST")"
  cp -r "$SRC" "$DST"
  echo "activated codex-peer-review into $DST"
fi
command -v codex >/dev/null 2>&1 \
  && echo "codex CLI: $(codex --version 2>/dev/null | head -1)" \
  || echo "codex CLI missing — install: npm i -g @openai/codex   then: codex auth login"
```

The `npm i -g @openai/codex` install and `codex auth login` are the user's to run
(global npm install and an interactive OpenAI sign-in). The skill stays inert until
the CLI is present; re-running `bootstrap.sh` then confirms the prerequisite. Note
in the final report whether the reviewer was activated and whether the CLI is ready.

## 4. Wire per-repo configuration

These two steps were previously part of `/oil`; they are configuration, not
project-layer indexing, so they live here. Both are idempotent: a re-run overwrites
only the keys it owns and leaves the rest of the settings untouched.

### Status line

A plugin cannot contribute a main `statusLine` — Claude Code reads `statusLine` only
from user/project/local settings, never from a plugin's bundled settings (a plugin's
`settings.json` honors only `agent` and `subagentStatusLine`). So the machine's status
line only appears once it is wired into the target repo's `.claude/settings.json`.

Point the wiring at the **stable shim**, not the versioned plugin cache. `bootstrap.sh`
installs/refreshes the script to `~/.claude/hooks/machine-statusline.mjs` (a
version-independent path) on every run, so a `/plugin update` ships the latest script
there automatically and the recorded command never rots. Do NOT write the
`${CLAUDE_PLUGIN_ROOT}/.claude/hooks/...` path: that token does not expand inside a
project `settings.json`, and its resolved value bakes in the version dir that an
update invalidates.

Merge a `statusLine` block into `<project>/.claude/settings.json` without
clobbering any other keys. If the file is absent, create it as `{}` first, then
merge. The block sets `statusLine.type` to `"command"`, `statusLine.command` to
`node ~/.claude/hooks/machine-statusline.mjs` (the shell that runs the status line
expands `~`), and `statusLine.refreshInterval` to `10`. Overwriting rather than
skipping keeps the command current if the stable path ever changes.

```bash
SETTINGS="$(git rev-parse --show-toplevel)/.claude/settings.json"
mkdir -p "$(dirname "$SETTINGS")"
[ -f "$SETTINGS" ] || printf '{}' > "$SETTINGS"
node -e '
  const fs = require("fs");
  const [file] = process.argv.slice(1);
  const s = JSON.parse(fs.readFileSync(file, "utf8"));
  s.statusLine = { type: "command", command: "node ~/.claude/hooks/machine-statusline.mjs", refreshInterval: 10 };
  fs.writeFileSync(file, JSON.stringify(s, null, 2) + "\n");
' "$SETTINGS"
```

### Required API keys

Some bundled MCP servers authenticate with an API key. This keeps a single list of
the machine's servers and the keys they require, so future required keys are added
in exactly one place.

**Required-key list:**

- **context7** (HTTP transport) — requires `CONTEXT7_API_KEY`.
- **kern**, **hub**, **pdf-reader**, **context-mode** — require no key.
- Local services (no key required): **board** — a local zero-dep Node daemon over a
  single repo-scoped JSON state file; it authenticates nothing and is never sent a key.

So today the only required key is `CONTEXT7_API_KEY`, and it is **optional**:
without it, context7 simply stays unauthenticated. Nothing else breaks, because the
`${CONTEXT7_API_KEY:-}` default in `plugin.json` `mcpServers` lets the whole config
still parse when the variable is unset.

For each required key: treat it as **already available** if it is present in the
process environment OR under `env` in `<project>/.claude/settings.local.json`. If
available, do nothing. If absent, ask the user once. If they provide it, merge it
into `settings.local.json` under `env`. If they decline, skip it and note that
context7 will be unauthenticated.

**Secrets handling — non-negotiable:**

- Write keys **only** to `.claude/settings.local.json`, which is gitignored. Never
  write a key into `plugin.json` or a committed `settings.json`.
- Never echo or log the key value. Read it from a shell variable and pass that
  variable to the node merge as an argument — never interpolate it into the script
  body. Do **not** use `sed`.
- Create `settings.local.json` as `{}` if absent, then merge without clobbering any
  other key or `env` entry. Keep the file valid JSON.

```bash
SETTINGS="$(git rev-parse --show-toplevel)/.claude/settings.local.json"
mkdir -p "$(dirname "$SETTINGS")"
[ -f "$SETTINGS" ] || printf '{}' > "$SETTINGS"
# $CONTEXT7_API_KEY holds the key (from the environment or the user); never printed.
node -e '
  const fs = require("fs");
  const [file, value] = process.argv.slice(1);
  const s = JSON.parse(fs.readFileSync(file, "utf8"));
  s.env = s.env || {};
  s.env.CONTEXT7_API_KEY = value;
  fs.writeFileSync(file, JSON.stringify(s, null, 2) + "\n");
' "$SETTINGS" "$CONTEXT7_API_KEY"
```

## 5. Specialize the project layer

Hand off to `/oil` to write `/.machine/` — the per-repo identity, project facts,
glossary, and persona panel. Invoke the `oil` skill now. If the user only wants
the install and config and not the project layer yet, they can stop before this
step; otherwise assemble is not finished until the layer exists.

## 6. Report

Give one compact status: which dependencies were installed vs already present, any
user-run plugin commands still pending, whether the status line and keys were
wired, and that `/oil` ran (or is the next step). Remind the user to restart
Claude Code or `/reload-plugins` so newly installed MCP servers and plugins load.

## Boundaries

- `/assemble` installs dependencies and writes **only** harness configuration
  (`.claude/settings.json` statusLine, `.claude/settings.local.json` env), the
  project layer via `/oil`, and — on explicit opt-in — the vendored
  `codex-peer-review` addon into the project's own `.claude/skills/`. It never
  edits the machine plugin's own content (`skills/`, `agents/`, `hooks/`) — that is
  the plugin's, updated via `/plugin update machine`.
- Dependency install logic has one source of truth: `scripts/bootstrap.sh`. Do not
  duplicate install commands here; call the script.
