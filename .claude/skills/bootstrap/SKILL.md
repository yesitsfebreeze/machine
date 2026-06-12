---
name: bootstrap
description: Clear /.proj and re-index it from the CURRENT project — scans the repo and seeds the project layer (agent identity, facts, glossary, persona panel) that specializes the portable .claude machine for this codebase. Run after copying the machine into a new repo or after the project changes shape.
---

# /bootstrap — re-index `/.proj` from the current project

`.claude/` is the **portable machine** (skills, agents, hooks, settings — identical
in every repo). `/.proj/` is the **project layer** — everything that diverges per
codebase. Bootstrap wipes `/.proj/` and regenerates it by reading the project, so a
freshly-copied machine specializes itself to *this* repo.

## Step 1 — Clear `/.proj/` (keep structure, wipe instance state)

```bash
cd "$(git rev-parse --show-toplevel 2>/dev/null || echo .)"
mkdir -p .proj/personas .proj/skills
rm -f .proj/agent.md .proj/project.md .proj/trello.json .proj/improve.json
rm -f .proj/*.lock .proj/*.lock.json
rm -f .proj/personas/*.md
printf 'category,term,definition\n' > .proj/glossary.csv
echo "cleared .proj — ready to re-index"
```

Keep `/.proj/.gitignore` and `/.proj/glossary.md` (the vocabulary doc) if present.
Everything else in `/.proj/` is regenerated below.

## Step 2 — Scan the project

Gather ground truth (use Read/Glob/Grep, and `mcp__kern__query` for prior decisions):

- **Identity & vision:** `README*`, `CLAUDE.md`, `docs/VISION.md` or `docs/**`.
- **Stack & build:** manifest files (`Cargo.toml`, `package.json`, `pyproject.toml`,
  `go.mod`, `CMakeLists.txt`, `Justfile`, `Makefile`), `.github/workflows/*`.
- **Shape:** top-level `src/`/module layout, entry points, the hot/critical path.
- **Platform:** OS, shell, target (desktop/embedded/web).
- **Domain laws:** any non-negotiable constraints stated in docs (real-time/safety,
  hardware limits, security boundaries, perf budgets).

If the project is thin or undocumented, ask the user 2-3 focused questions rather
than guessing identity or laws.

## Step 3 — Seed the project layer

Write these files from the scan. Keep each terse and specific to THIS repo:

1. **`/.proj/project.md`** — facts: name, domain (one line), stack, platform,
   target, authoritative spec path, key paths (hot path, entry points, mapping
   docs), build + test + quality-gate commands, CI path.

2. **`/.proj/agent.md`** — the project half of the agent (read by
   `.claude/agents/default.md`): *What this project is*, *Project law* (the binding
   domain rules from the scan — as hard as machine law), *Domain idioms*, the
   *persona panel* roster, and *build/verify* commands.

3. **`/.proj/glossary.csv`** — seed rows (`category,term,definition`) for ambiguous
   domain terms found in docs/code. Header-only is fine if none are clear yet.

4. **`/.proj/personas/` + `/.proj/personas.md`** — a review panel tuned to the
   project's real risk surfaces (e.g. concurrency, the critical path, UX, security,
   storage). One `*.md` per reviewer (name, role, what they catch); `personas.md`
   indexes them with `**File:** .proj/personas/<name>.md` pointers. If the domain is
   unclear, write an empty `personas.md` stub and tell the user to author the panel.

## Step 4 — Report

List what was wiped and what was seeded (the files + a one-line summary of each).
Confirm the machine is now specialized to this repo and `/personas`, `/gate`,
glossary discipline, and the dispatch table are live.

**Never touch the machine** (`skills/`, `agents/`, `hooks/`, `settings.json`,
`rules/`) — bootstrap only writes under `/.proj/`.
