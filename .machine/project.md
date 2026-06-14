# Project facts — the machine

- **Name:** the machine
- **Domain (one line):** a portable, project-agnostic Claude Code agent setup — agents, skills, hooks, rules, settings — that installs as `<project>/.claude/` and specializes itself per repo via `/oil-me`.
- **Stack:** Markdown instruction documents (agents, skills, commands, rules, output-styles); Node ESM hooks (`*.mjs`); JSON config (`settings.json`); CSV/Markdown glossary.
- **Platform:** Windows, PowerShell 7+ shell, git-backed.
- **Target:** Claude Code plugin named `machine` (manifest in `.claude-plugin/`); also usable vendored as `<project>/.claude/`. This repo root *is* the machine source; `/.machine/` is the project layer (never shipped).
- **Authoritative spec:** `.claude-plugin/plugin.json` (plugin manifest) + `.claude/skills/oil-me/SKILL.md` (re-index protocol) + `.claude/agents/default.md` (machine law) are the canonical truth.

## Key paths
- `.claude/agents/default.md` — the eager-generalist default agent (reads `/.machine`)
- `.claude/agents/*.md` — 23 dispatch agents (`expert-*`, `manager-*`, `builder-*`); resolved by `name:` frontmatter, not path
- `.claude/skills/` — 21 skill dirs; `name:` frontmatter must match dir name
- `.claude/rules/coding-standards.md` + `.claude/rules/languages/*` (16 language rules)
- `.claude/hooks/personas.mjs` (Stop hook), `.claude/hooks/statusline.mjs`
- `.claude/skills/oil-me/` — `/oil-me`, re-indexes `/.machine` from the current repo (install/update via `/plugin`)
- `.claude-plugin/plugin.json` + `.claude-plugin/marketplace.json` — plugin + marketplace manifests (plugin.json lists agent files + skill dirs explicitly; regenerate when adding/removing agents or skills)
- `.claude/hooks/hooks.json` — plugin hook manifest (`${CLAUDE_PLUGIN_ROOT}` paths); plugin `settings` are inline in plugin.json
- `.claude/settings.json` — self-host hook wiring, env, `agent: default` (NOT shipped by the plugin — keeps `bypassPermissions` out of installs)
- `.machine/improve.json` — **live worklist** for the ongoing `/improve` loop (tracked in git; outstanding work only)

## Build / test / quality gate
There is no compile step. "Build" = configuration integrity:
- **Hooks parse:** `node --check .claude/hooks/personas.mjs` and `.claude/hooks/statusline.mjs`
- **Settings parse:** `Get-Content .claude/settings.json | ConvertFrom-Json` (must not throw)
- **Dispatch integrity:** every agent `name:` is unique; every agent `skills:` ref resolves to a `.claude/skills/<name>/` dir
- **Standards:** `.claude/rules/coding-standards.md` (English-only instructions, no emoji, CLAUDE.md ≤ 40k chars, thin-command pattern < 20 LOC)
- **Gate skill:** `/gate` detects the stack; for this repo it runs the parse + integrity checks above.

## CI
None configured (no `.github/workflows`). Git is the only safety net — commit per verified slice.
