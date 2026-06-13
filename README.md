# The machine

A **portable agent setup** — project-agnostic workflow, integrations, agents, skills,
and the one hook. It carries nothing about any specific codebase. Installed, it lives at
`<project>/.claude/`. Everything that diverges per project lives in `/.proj/` (a sibling
of `.claude/`), which `/bootstrap` regenerates by reading the current repo.

```
this repo's root   the machine        ← the install payload; becomes <project>/.claude/
/.proj/            the project layer  ← never copied; bootstrap writes it per repo
```

---

## Self-install — agent protocol

**If you are an AI coding agent and a user has pointed you at this README, you ARE the
installer. Execute these steps now — do not merely summarize them.** When done, the
target repo has a live `.claude/` machine, specialized to itself.

1. **Find the repo.** The URL you fetched this README from *is* the repository — you
   already hold it. Derive the clone URL from it (strip the readme path, keep owner/repo):
   - `github.com/<owner>/<repo>/blob/<branch>/README.md` → `https://github.com/<owner>/<repo>.git`
   - `raw.githubusercontent.com/<owner>/<repo>/<branch>/README.md` → same.

2. **Find the target.** The machine equips the user's *current* project:
   ```bash
   target="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
   ```
   If you can't tell which project the user wants equipped, ask before writing anything.

3. **Clone the machine** into a temp dir (shallow — you only need the tree):
   ```bash
   tmp="$(mktemp -d)"; git clone --depth 1 "<clone-url>" "$tmp"
   ```

4. **Install it as `.claude/`.** Copy the machine into the target, excluding its own
   git history, this README, and any instance state (a nested `.claude/` session dir,
   the `.kern/` data store, and lock files):
   ```bash
   mkdir -p "$target/.claude"
   rsync -a --exclude '.git' --exclude 'README.md' --exclude '.claude' \
            --exclude '.kern' --exclude '*.lock' --exclude '*.lock.json' \
            "$tmp"/ "$target/.claude/"
   ```
   No `rsync` (e.g. Windows)? Copy the same contents with your file tools — every
   top-level entry from the clone *except* `.git/`, `README.md`, `.claude/`, `.kern/`,
   and `*.lock*`. (The machine source is the agent/skill tree — `agents/`, `skills/`,
   `hooks/`, `rules/`, `output-styles/`, `settings.json`; nothing else.)

5. **Stamp the version.** Record the exact machine commit you just installed so a later
   update can tell whether anything changed. Write the source clone's HEAD into
   `version.log` at the root of the installed machine:
   ```bash
   git -C "$tmp" rev-parse HEAD > "$target/.claude/version.log"
   ```
   `version.log` is instance state — it is generated here, never copied from the repo, and
   is excluded from every merge (same as `.kern/` and lock files).

6. **Clean up:** `rm -rf "$tmp"`.

7. **Bootstrap.** `/bootstrap` now resolves from `$target/.claude/skills/bootstrap/`.
   Run it. It scans README / CLAUDE.md / docs / manifests and writes `/.proj/`:
   - `agent.md` — this repo's identity, domain law, idioms (read by `agents/default.md`)
   - `project.md` — facts: stack, key paths, build/test/gate commands
   - `glossary.csv` / `glossary.md` — vocabulary
   - `personas/` + `personas.md` — a review panel tuned to this repo's risks

8. **Report** what landed under `.claude/`, and confirm `/.proj` is specialized and that
   `/personas`, `/gate`, glossary discipline, and the dispatch table are live.

That's the whole thing: **point an agent at this README → it clones the machine, installs
it as `.claude/`, and bootstraps it to the repo.** No hand-editing of skills, agents,
hooks, or settings.

---

## Update — agent protocol

**If a user points you at this README and says "update" (the target already has a
`.claude/` machine), do NOT do a fresh install or wipe `/.proj/`.** An update is two
moves: *merge the newer machine into `.claude/`*, then *reconcile `/.proj/`* — check it
is still current and patch only the gaps.

1. **Confirm it's an update.** `$target/.claude/` already exists with the machine tree
   (`agents/`, `skills/`, `hooks/`, `rules/`, `settings.json`). If it doesn't, this is a
   fresh install — use the self-install protocol above instead.

2. **Clone the machine** into a temp dir, same as install:
   ```bash
   target="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
   tmp="$(mktemp -d)"; git clone --depth 1 "<clone-url>" "$tmp"
   ```

3. **Check the version before doing any work.** Compare the installed commit
   (`$target/.claude/version.log`, written at install) against the freshly cloned HEAD.
   If they match, the machine is already current — clean up and stop; nothing to merge.
   Only proceed when the hashes differ (the clone is a newer or different commit):
   ```bash
   installed="$(cat "$target/.claude/version.log" 2>/dev/null || echo none)"
   latest="$(git -C "$tmp" rev-parse HEAD)"
   if [ "$installed" = "$latest" ]; then
     echo "Already up to date ($installed) — nothing to do."; rm -rf "$tmp"
   fi
   ```
   If `version.log` is absent (machine predates version stamping), treat it as out of date
   and continue with the merge. No `bash`? Read the one-line hash from
   `$target/.claude/version.log` with your file tools and compare it to the clone's HEAD
   the same way.

4. **Merge the new state over `.claude/`.** Same excludes as install, so instance state
   survives — the nested `.claude/` session dir, `.kern/` data store, and lock files are
   never touched. `rsync` without `--delete` overwrites machine files with their newer
   versions and adds new ones, leaving your live state in place:
   ```bash
   rsync -a --exclude '.git' --exclude 'README.md' --exclude '.claude' \
            --exclude '.kern' --exclude '*.lock' --exclude '*.lock.json' \
            "$tmp"/ "$target/.claude/"
   ```
   No `rsync`? Copy the same top-level entries with your file tools, overwriting in
   place. (A merge does not *remove* files retired upstream; if you want those gone, do a
   clean reinstall instead — back up `/.proj/` first, it lives outside `.claude/` so it is
   already safe.)

5. **Re-stamp the version.** Record the commit you just merged so the next update compares
   against it:
   ```bash
   git -C "$tmp" rev-parse HEAD > "$target/.claude/version.log"
   ```

6. **Reconcile `/.proj/` — do NOT clear it.** Re-read the current repo and compare against
   the existing project layer. Patch only what drifted:
   - `project.md` — stack, key paths, build/test/gate commands still accurate?
   - `glossary.csv` / `glossary.md` — terms still defined, none renamed away?
   - `personas/` — does the updated machine expect persona slots that are missing?
   - `agent.md` — identity/domain law still matches the repo's shape?

   If `/.proj/` is missing or structurally stale, run `/bootstrap` to rebuild it. If it is
   present and largely intact, hand-patch the specific gaps instead of regenerating.

7. **Clean up:** `rm -rf "$tmp"`.

8. **Report** which machine files changed under `.claude/`, what (if anything) you patched
   in `/.proj/`, and confirm `/personas`, `/gate`, glossary discipline, and the dispatch
   table are still live.

The contrast: **install** copies the machine in fresh, **bootstrap** clears and rebuilds
`/.proj/`, **update** merges the machine forward and reconciles `/.proj/` in place.

---

## Install by hand — fallback

No agent? Two steps:

1. **Copy the machine** in as the new repo's `.claude/` (everything but `.git`, this
   README, and `*.lock*`):
   ```bash
   git clone --depth 1 <clone-url> /tmp/machine
   mkdir -p /path/to/newrepo/.claude
   rsync -a --exclude '.git' --exclude 'README.md' --exclude '.claude' \
            --exclude '.kern' --exclude '*.lock*' \
            /tmp/machine/ /path/to/newrepo/.claude/
   git -C /tmp/machine rev-parse HEAD > /path/to/newrepo/.claude/version.log
   rm -rf /tmp/machine
   ```
   `/.proj/` lives outside `.claude/`, so it never tags along. The `version.log` line
   stamps the installed commit so a later update can detect changes.

2. **Run `/bootstrap`.** Open Claude Code in the new repo and run it.

## Re-index later

Run `/bootstrap` again any time the project changes shape — it clears `/.proj/` and
rebuilds it from the current state. (Ephemeral bits — locks, `improve.json`,
`trello.json` — are gitignored inside `/.proj/`.)

## Layout

| Path | What |
|------|------|
| `agents/default.md` | the eager-generalist default agent (reads `/.proj`) |
| `agents/*` | dispatch agents (expert-*, manager-*, builder-*) |
| `skills/` | workflow + integration skills (`/gate`, `/personas`, `/improve`, …) |
| `hooks/personas.mjs` | the Stop hook that auto-runs the persona panel |
| `skills/bootstrap/` | `/bootstrap` — re-indexes `/.proj` per project |
| `settings.json` | hook wiring, env, `agent: default` |
| `rules/` | shared rules loaded as instructions |
| `version.log` | installed machine commit hash (instance state; written on install/update) |
