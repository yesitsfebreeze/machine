---
name: oil-me
description: Oil the machine — the single lifecycle command. Installs the machine into a new repo, updates an existing machine from its source, and re-indexes /.proj to specialize the portable .claude to THIS codebase. Replaces /bootstrap. Run after copying the machine into a new repo, to pull machine updates, or whenever the project changes shape. Trigger: "/oil-me", "oil the machine", "bootstrap", "update the machine", "init the machine", "re-index /.proj".
---

# /oil-me — install, update & re-index the machine

`/oil-me` is the machine's one lifecycle command. The full protocol lives in
**`.claude/INSTRUCTIONS.md`** (single source of truth); this skill detects the current
state and executes the matching section of that file.

## Procedure

1. **Read `.claude/INSTRUCTIONS.md`.** It is the authoritative protocol — install, update,
   and re-index, plus the self-update directive on its first line.

2. **Self-refresh the instructions.** Take the `<url>` on line one of
   `.claude/INSTRUCTIONS.md` (filled in at install). Fetch that remote URL. If the remote
   body differs from the local file, overwrite the local file with the remote content, then
   re-substitute `<url>` on line one with the real raw URL so it keeps self-refreshing. If
   line one still holds the literal `<url>` placeholder (never installed, or you are running
   inside the machine's own source repo), skip this step — there is nothing to pull from.

3. **Detect the state and run the matching section:**
   - **No `.claude/` machine in the target** → run **Install**, then **Re-index `/.proj/`**.
   - **`.claude/` already present** → run **Update**, then **Re-index `/.proj/`** (reconcile
     in place, never wipe).
   - **Machine fine, only the project changed shape** → run **Re-index `/.proj/`** only.

   The clone URL is derived from the line-one `<url>` (strip the file path, keep owner/repo).

4. **Report** what changed under `.claude/` and `/.proj/`, and confirm `/personas`, `/gate`,
   glossary discipline, and the dispatch table are live.

## Boundaries

- Install/Update write under `.claude/` and stamp `.claude/version.log` (instance state).
- Re-index writes **only** under `/.proj/` — never touch `skills/`, `agents/`, `hooks/`,
  `settings.json`, or `rules/` during re-index.
- `version.log` and the filled-in line-one `<url>` are instance state — never committed to
  the machine's source repo.
