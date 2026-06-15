---
name: oil-me
description: Oil the machine — re-index /.machine to specialize the portable machine to THIS codebase. Install and update are handled by the Claude Code plugin system (`/plugin`); this skill owns the per-repo project layer. Run after installing the machine plugin in a new repo, or whenever the project changes shape. Trigger: "/oil-me", "oil the machine", "re-index", "update the machine", "init the machine", "re-index /.machine".
---

# /oil-me — re-index the project layer

The machine ships as a Claude Code **plugin** named `machine`. Install and update
are the plugin system's job; `/oil-me` owns the one thing the plugin cannot: the
per-repo **project layer** `/.machine/`, which specializes the portable machine to
*this* codebase.

## Install & update (plugin system — not this skill)

```
/plugin marketplace add yesitsfebreeze/machine
/plugin install machine@machine
/plugin update machine
```

Installing namespaces every component (`machine:<skill>`, `machine:<agent>`).
The project layer `/.machine/` is **never** part of the plugin — it diverges per
repo and is always generated locally by this skill. Rules (`.claude/rules/`) and
harness policy such as `permissions.defaultMode` stay the installing repo's call.

After `/plugin install` (or after the repo changes shape), run `/oil-me`.

### Bundled MCP servers & companion plugins

The plugin's `.mcp.json` ships `kern` (memory), `mesh` (fleet coordination — needs
`cargo install --path mesh`), `context7` (library docs — needs `CONTEXT7_API_KEY`),
and `pdf-reader` (PDF extraction). Two live companion plugins
are routed to but not vendored — `context-mode` (`mksglu/context-mode`) and
`git-fs` (`yesitsfebreeze/git-fs`); install them separately if a repo wants them.
The default agent's toolbelt section is the single source of truth for when to use
each.

## Re-index `/.machine/`

Specialize the portable machine to *this* repo. On first run, regenerate from
scratch. If a project layer already exists, **reconcile in place** — patch only
what drifted, never blow away a working layer.

### First run — regenerate

1. **Clear `/.machine/` (keep structure, wipe instance state):**
   ```bash
   cd "$(git rev-parse --show-toplevel 2>/dev/null || echo .)"
   mkdir -p .machine/personas .machine/skills
   rm -f .machine/agent.md .machine/project.md .machine/trello.json .machine/improve.json
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

4. **Wire the status line into this project** — see "Wire the status line into this project" below.

5. **Collect the required API keys** — see "Collect required API keys" below.

### Re-run — reconcile in place

Re-read the current repo and compare against the existing project layer. Patch only what
drifted; do **not** clear `/.machine/`:
- `project.md` — stack, key paths, build/test/gate commands still accurate?
- `glossary.csv` / `glossary.md` — terms still defined, none renamed away?
- `personas/` — does the updated machine expect persona slots that are missing?
- `agent.md` — identity/domain law still matches the repo's shape?
- status line — re-wire it; see "Wire the status line into this project" below.
- required API keys — re-check them; see "Collect required API keys" below.

If `/.machine/` is missing or structurally stale, regenerate it (the first-run path above).
If present and largely intact, hand-patch the specific gaps.

## Wire the status line into this project

A plugin cannot contribute a main `statusLine`, so the machine's status line only
appears once it is wired into the target repo. This per-repo step performs that
wiring by injecting a `statusLine` block into the TARGET project's
`.claude/settings.json`.

Resolve the installed plugin's status line script at `${CLAUDE_PLUGIN_ROOT}/.claude/hooks/statusline.mjs`.
The `${CLAUDE_PLUGIN_ROOT}` token expands here in skill content, so capture the
RESOLVED absolute path. Write that resolved path as a LITERAL string into the
project settings, because the `${CLAUDE_PLUGIN_ROOT}` token does NOT expand inside
a project `settings.json`.

Merge a `statusLine` block into `<project>/.claude/settings.json` without
clobbering any other keys. If the file is absent, create it as `{}` first, then
merge. The result must remain valid JSON. The merged block sets `statusLine.type`
to `"command"`, `statusLine.command` to `node "<resolved-abs-path>"` (the resolved
absolute path quoted inside the command string), and `statusLine.refreshInterval`
to `10`.

This step is idempotent: on every re-run, overwrite only the `statusLine` key and
leave the rest of the settings untouched. Overwriting rather than skipping means a
plugin update that moves the install directory self-heals the recorded path on the
next run.

```bash
SETTINGS="$(git rev-parse --show-toplevel)/.claude/settings.json"
SCRIPT="${CLAUDE_PLUGIN_ROOT}/.claude/hooks/statusline.mjs"
mkdir -p "$(dirname "$SETTINGS")"
[ -f "$SETTINGS" ] || printf '{}' > "$SETTINGS"
node -e '
  const fs = require("fs");
  const [file, script] = process.argv.slice(1);
  const s = JSON.parse(fs.readFileSync(file, "utf8"));
  s.statusLine = { type: "command", command: `node "${script}"`, refreshInterval: 10 };
  fs.writeFileSync(file, JSON.stringify(s, null, 2) + "\n");
' "$SETTINGS" "$SCRIPT"
```

## Collect required API keys

Some bundled MCP servers authenticate with an API key. This step keeps a single
list of the machine's servers and the keys they require, so future required keys
are added in exactly one place rather than scattered across the skill. For each
key in the list, oiling detects whether it is already available and, if not, asks
the user once and records it in the gitignored local settings.

### Required-key list

The machine's bundled MCP servers and their key requirements:

- **context7** (HTTP transport) — requires `CONTEXT7_API_KEY`.
- **kern**, **mesh**, **pdf-reader**, **git-fs** — require no key.

So today the only required key is `CONTEXT7_API_KEY`, and it is **optional**:
without it, context7 simply stays unauthenticated and unavailable. Nothing else
breaks, because the `${CONTEXT7_API_KEY:-}` default in `.mcp.json` lets the whole
config still parse when the variable is unset. Add future keys to this list and
to the recipe below; the rest of the step needs no other change.

### Detect, ask, record

For each required key, on both first run and re-run, do the following. Treat the
key as **already available** if it is present in the process environment OR under
`env` in `<project>/.claude/settings.local.json`. If it is already available, do
nothing. If it is absent, ask the user for it. If the user provides it, merge it
into `<project>/.claude/settings.local.json` under `env.CONTEXT7_API_KEY`. If the
user declines, skip it and note that context7 will be unauthenticated.

### Secrets handling

These rules are non-negotiable:

- Write keys **only** to `.claude/settings.local.json`, which is gitignored. Never
  write a key into `.mcp.json` or into a committed `settings.json`.
- Never echo or log the key value.
- Create `settings.local.json` as `{}` if it is absent, then merge.
- Merge without clobbering: preserve every other key (for example
  `permissions.allow`) and every other `env` entry.
- Keep the file valid JSON.
- This step is idempotent: a re-run overwrites only `env.CONTEXT7_API_KEY` and
  leaves the rest of the settings untouched.

### Recipe

Read the key from a shell variable that already holds it (for example an
environment variable, or a value the user just supplied) and pass that variable
to the node merge as an argument, so the value is never interpolated into the
script body, printed, or logged. The node merge mirrors the status-line block: it
loads the JSON, ensures an `env` object exists, sets the one key from `argv`, and
writes the file back. Do **not** use `sed` for this.

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

## Report

Report what changed under `/.machine/`, and confirm `/personas`, `/gate`, glossary
discipline, and the dispatch table are live.

## Boundaries

- `/oil-me` writes **only** under `/.machine/`. Never touch the machine itself
  (`skills/`, `agents/`, `hooks/`, `settings.json`, `rules/`) during re-index —
  that is the plugin's content, updated via `/plugin update machine`.
