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

### Re-run — reconcile in place

Re-read the current repo and compare against the existing project layer. Patch only what
drifted; do **not** clear `/.machine/`:
- `project.md` — stack, key paths, build/test/gate commands still accurate?
- `glossary.csv` / `glossary.md` — terms still defined, none renamed away?
- `personas/` — does the updated machine expect persona slots that are missing?
- `agent.md` — identity/domain law still matches the repo's shape?

If `/.machine/` is missing or structurally stale, regenerate it (the first-run path above).
If present and largely intact, hand-patch the specific gaps.

## Report

Report what changed under `/.machine/`, and confirm `/personas`, `/gate`, glossary
discipline, and the dispatch table are live.

## Boundaries

- `/oil-me` writes **only** under `/.machine/`. Never touch the machine itself
  (`skills/`, `agents/`, `hooks/`, `settings.json`, `rules/`) during re-index —
  that is the plugin's content, updated via `/plugin update machine`.
