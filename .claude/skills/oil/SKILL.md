---
name: oil
description: Oil the machine — re-index /.machine to specialize the portable machine to THIS codebase, then fire /mine to equip it with best-fit tools. Install and update are handled by the Claude Code plugin system (`/plugin`); this skill owns the per-repo project layer. Run after installing the machine plugin in a new repo, or whenever the project changes shape. Trigger: "/oil", "oil the machine", "re-index", "update the machine", "init the machine", "re-index /.machine".
---

# /oil — re-index the project layer

The machine ships as a Claude Code **plugin** named `machine`. `/oil` owns the
one thing the plugin and drill's bring-up do not: the per-repo **project layer**
`/.machine/`, which specializes the portable machine to *this* codebase. It
diverges per repo and is always generated locally by this skill; it is **never**
part of the plugin.

Install, dependency bootstrap, and configuration are **drill's bring-up** job
(its `references/assemble.md`, which reuses `just bootstrap` / `scripts/bootstrap.sh`) —
it installs `kern`, `mesh`, the `git-fs` plugin, and the MCP prerequisites, wires the
status line and API keys, then invokes this skill. Drill runs that bring-up first in a
fresh repo. Run `/oil` on its own whenever the project changes shape and only the
project layer needs re-indexing.

## Re-index `/.machine/`

Specialize the portable machine to *this* repo. On first run, regenerate from
scratch. If a project layer already exists, **reconcile in place** — patch only
what drifted, never blow away a working layer.

### First run — regenerate

1. **Clear `/.machine/` (keep structure, wipe instance state):**
   ```bash
   cd "$(git rev-parse --show-toplevel 2>/dev/null || echo .)"
   mkdir -p .machine/personas .machine/skills
   rm -f .machine/agent.md .machine/project.md .machine/taskboard.json .machine/improve.json
   rm -f .machine/*.lock .machine/*.lock.json
   rm -f .machine/personas/*.md
   printf 'category,term,definition\n' > .machine/glossary.csv
   ```
   Keep `/.machine/.gitignore` and `/.machine/glossary.md` if present.

2. **Scan the project** (Read/Glob/Grep, plus `mcp__kern__query` for prior decisions):
   - **Identity & vision:** `README*`, `CLAUDE.md`, `docs/VISION.md` or `docs/**`.
   - **Stack & build:** manifests (`Cargo.toml`, `package.json`, `pyproject.toml`, `go.mod`,
     `CMakeLists.txt`, `Justfile`, `Makefile`), `.github/workflows/*`.
   - **Shape:** top-level module layout, entry points, the hot/critical path.
   - **Platform:** OS, shell, target (desktop/embedded/web).
   - **Domain laws:** non-negotiable constraints in docs (real-time/safety, hardware limits,
     security boundaries, perf budgets).

   If the project is thin or undocumented, ask the user 2-3 focused questions rather than
   guessing identity or laws.

3. **Seed the project layer** — terse, specific to THIS repo:
   - **`/.machine/project.md`** — facts: name, domain (one line), stack, platform, target,
     authoritative spec path, key paths (hot path, entry points, mapping docs), build + test
     + quality-gate commands, CI path.
   - **`/.machine/agent.md`** — the project half of the agent (read by
     `.claude/agents/default.md`): *What this project is*, *Project law* (binding domain
     rules — as hard as machine law), *Domain idioms*, the *persona panel* roster, and
     *build/verify* commands.
   - **`/.machine/glossary.csv`** — seed rows (`category,term,definition`) for ambiguous domain
     terms. Header-only is fine if none are clear yet.
   - **`/.machine/personas/` + `/.machine/personas.md`** — a review panel tuned to the project's
     real risk surfaces. One `*.md` per reviewer (name, role, what they catch);
     `personas.md` indexes them with `**File:** .machine/personas/<name>.md` pointers. If the
     domain is unclear, write an empty `personas.md` stub and tell the user to author it.

4. **Ensure the status line is wired** (self-heal). A plugin cannot contribute a main
   `statusLine`, so it must live in the repo's `.claude/settings.json`. `/oil` is the
   advertised post-install entry point, so guarantee a working statusbar here rather
   than assuming drill's bring-up ran. Both steps are idempotent:
   ```bash
   # a) ensure the stable shim exists (bring-up's bootstrap also installs/refreshes it)
   SHIM="$HOME/.claude/hooks/machine-statusline.mjs"
   mkdir -p "$(dirname "$SHIM")"
   [ -f "$SHIM" ] || cp "${CLAUDE_PLUGIN_ROOT}/.claude/hooks/statusline.mjs" "$SHIM"
   # b) point this repo's settings at the stable shim (never the versioned cache path)
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
   API keys remain bring-up's job (references/assemble.md); only the status line is
   self-healed here, because it is the one piece a user reasonably expects `/oil` to leave working.

### Re-run — reconcile in place

Re-read the current repo and compare against the existing project layer. Patch only what
drifted; do **not** clear `/.machine/`:
- `project.md` — stack, key paths, build/test/gate commands still accurate?
- `glossary.csv` / `glossary.md` — terms still defined, none renamed away?
- `personas/` — does the updated machine expect persona slots that are missing?
- `agent.md` — identity/domain law still matches the repo's shape?

If `/.machine/` is missing or structurally stale, regenerate it (the first-run path above).
If present and largely intact, hand-patch the specific gaps. Re-run the status-line
self-heal step above (it is idempotent); to re-check API keys, re-run drill's bring-up
(or `just bootstrap`).

## Refresh the env cache

Materialize `/.machine/ENV.md` now so this same session can resolve machine paths
(it is otherwise written at the next SessionStart). Run the hook directly — it
derives the install-correct plugin root from its own location and writes the cache:

```bash
node "${CLAUDE_PLUGIN_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}/.claude/hooks/ignite.mjs" >/dev/null 2>&1 || true
```

That fallback is valid only *here* (where the plugin root equals the repo root, or
`${CLAUDE_PLUGIN_ROOT}` is set) and only to *locate the hook* — the hook self-corrects
the path it records. Everything downstream reads `MACHINE_MINE` / `MACHINE_PLUGIN_ROOT`
from the cache, never a git guess.

## Then mine — equip the machine

The project layer is now current, so `/oil` knows what this repo is. Hand off to
`/mine` to act on it: once the re-index above is done, invoke the `mine` skill.
`/oil` scrapes and updates the **project**; `/mine` scrapes and updates the
**tools** — it surveys the mine graph against the freshly-indexed `/.machine/` and
slots in the best-fit addons. They are one motion: oil specializes, mine equips.

Skip the hand-off only if the user scoped `/oil` to the project layer alone, or the
mine kit is genuinely absent — and check the right place before declaring it absent:
`$MACHINE_MINE` from `/.machine/ENV.md` (the plugin payload), **not** the target
project's CWD. A repo that only installed the machine has no `mine/` at its own root,
which is not the same as the kit being absent.

## Report

Report what changed under `/.machine/`, and confirm `/personas`, `/gate`, glossary
discipline, and the dispatch table are live.

## Boundaries

- `/oil` writes under `/.machine/`, plus the single `statusLine` key in the **target
  repo's** `.claude/settings.json` (the self-heal step above) and the stable shim at
  `~/.claude/hooks/`. It never touches the **machine plugin's own** content
  (`skills/`, `agents/`, `hooks/`, the plugin's `settings.json`, `rules/`) during
  re-index — that is the plugin's content, updated via `/plugin update machine`.
