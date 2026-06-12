# Project facts — the machine

- **Name:** the machine
- **Domain (one line):** a portable, project-agnostic Claude Code agent setup — agents, skills, hooks, rules, settings — that installs as `<project>/.claude/` and specializes itself per repo via `/bootstrap`.
- **Stack:** Markdown instruction documents (agents, skills, commands, rules, output-styles); Node ESM hooks (`*.mjs`); JSON config (`settings.json`); CSV/Markdown glossary.
- **Platform:** Windows (`E:\dev\tempalte`), PowerShell 7+ shell, git-backed.
- **Target:** install payload — the tree becomes `<project>/.claude/`. This repo root *is* the machine source; `/.proj/` is the project layer (never copied).
- **Authoritative spec:** `README.md` (install protocol + layout) + `.claude/agents/default.md` (machine law) are the canonical truth.

## Key paths
- `.claude/agents/default.md` — the eager-generalist default agent (reads `/.proj`)
- `.claude/agents/*.md` — 23 dispatch agents (`expert-*`, `manager-*`, `builder-*`); resolved by `name:` frontmatter, not path
- `.claude/skills/` — 21 skill dirs; `name:` frontmatter must match dir name
- `.claude/rules/coding-standards.md` + `.claude/rules/languages/*` (16 language rules)
- `.claude/hooks/personas.mjs` (Stop hook), `.claude/hooks/statusline.mjs`
- `.claude/commands/bootstrap.md` — the one per-project command
- `.claude/settings.json` — hook wiring, env, `agent: default`
- `.proj/improve.json` — **live worklist** for the ongoing unification `/loop` (gitignored, ephemeral)

## Build / test / quality gate
There is no compile step. "Build" = configuration integrity:
- **Hooks parse:** `node --check .claude/hooks/personas.mjs` and `.claude/hooks/statusline.mjs`
- **Settings parse:** `Get-Content .claude/settings.json | ConvertFrom-Json` (must not throw)
- **Dispatch integrity:** every agent `name:` is unique; every agent `skills:` ref resolves to a `.claude/skills/<name>/` dir
- **Standards:** `.claude/rules/coding-standards.md` (English-only instructions, no emoji, CLAUDE.md ≤ 40k chars, thin-command pattern < 20 LOC)
- **Gate skill:** `/gate` detects the stack; for this repo it runs the parse + integrity checks above.

## CI
None configured (no `.github/workflows`). Git is the only safety net — commit per verified slice.
