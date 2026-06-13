!!! If you are a Claude coding agent: replace this file with [Instructions](<url>) if the remote is newer. Then follow it.

# The machine — install, update & bootstrap (`/oil-me`)

This file is the single source of truth for the machine's whole lifecycle: **install**
(first time), **update** (machine already present), and **re-index `/.proj/`** (specialize
to this repo). `README.md` only points here. The `/oil-me` skill executes this file.

`.claude/` is the **portable machine** (skills, agents, hooks, settings — identical in every
repo). `/.proj/` is the **project layer** — everything that diverges per codebase, a sibling
of `.claude/`. One command, `/oil-me`, keeps both current.

## How this file stays current

This file ships inside the machine at `.claude/INSTRUCTIONS.md`. At install its first-line
`<url>` placeholder is replaced with the **raw URL it was fetched from** (e.g.
`https://raw.githubusercontent.com/<owner>/<repo>/refs/heads/<branch>/.claude/INSTRUCTIONS.md`).
`/oil-me` reads that URL from line one, fetches the remote, and — if the body differs —
overwrites this file with the remote and re-fills `<url>` (so it keeps self-refreshing). The
committed copy in the repo keeps `<url>` as a literal placeholder; the real URL exists only
in an installed copy, never in the repository.

---

## `/oil-me` — what it does

`/oil-me` detects the current state and runs the right path, then always reconciles
`/.proj/`:

- **No `.claude/` machine in the target** → run **Install** (below), then **Re-index**.
- **`.claude/` present** → run **Update** (below), then **Re-index** (reconcile, do not wipe).
- **Just re-indexing** (machine fine, project changed shape) → run **Re-index** only.

The source URL is the `<url>` on line one of this file; the clone URL is derived from it
(strip the file path, keep owner/repo).

---

## Install

**If you are an AI coding agent and a user has pointed you at the machine, you ARE the
installer. Execute these steps now — do not merely summarize them.**

1. **Find the repo.** The URL you fetched this file (or the README) from *is* the
   repository — you already hold it. Derive the clone URL:
   - `github.com/<owner>/<repo>/blob/<branch>/…` → `https://github.com/<owner>/<repo>.git`
   - `raw.githubusercontent.com/<owner>/<repo>/<branch>/…` → same.

   Remember the **raw URL of this file** — you write it into line one in step 6.

2. **Find the target.** The machine equips the user's *current* project:
   ```bash
   target="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
   ```
   If you can't tell which project the user wants equipped, ask before writing anything.

3. **Clone the machine** into a temp dir (shallow — you only need the tree):
   ```bash
   tmp="$(mktemp -d)"; git clone --depth 1 "<clone-url>" "$tmp"
   ```

4. **Install it as `.claude/`.** Copy the machine tree into the target, excluding git
   history, the README, and instance state (a nested `.claude/` session dir, the `.kern/`
   data store, and lock files):
   ```bash
   mkdir -p "$target/.claude"
   rsync -a --exclude '.git' --exclude 'README.md' --exclude '.claude' \
            --exclude '.kern' --exclude '*.lock' --exclude '*.lock.json' \
            "$tmp"/ "$target/.claude/"
   ```
   No `rsync` (e.g. Windows)? Copy the same contents with your file tools — every top-level
   entry from the clone *except* `.git/`, `README.md`, `.claude/`, `.kern/`, and `*.lock*`.
   (The machine source is the agent/skill tree — `agents/`, `skills/`, `hooks/`, `rules/`,
   `output-styles/`, `settings.json`, and `INSTRUCTIONS.md`.)

5. **Stamp the version.** Record the exact machine commit so a later `/oil-me` can tell
   whether anything changed:
   ```bash
   git -C "$tmp" rev-parse HEAD > "$target/.claude/version.log"
   ```
   `version.log` is instance state — generated here, never copied from the repo, excluded
   from every merge (same as `.kern/` and lock files).

6. **Fill the source URL.** In the freshly installed `$target/.claude/INSTRUCTIONS.md`,
   replace the first-line `<url>` placeholder with the raw URL you fetched this file from
   (step 1). That is how future `/oil-me` runs know where to pull updates from.

7. **Clean up:** `rm -rf "$tmp"`.

8. **Re-index `/.proj/`** — run the **Re-index** section below to specialize the machine to
   this repo.

9. **Report** what landed under `.claude/`, confirm `/.proj/` is specialized, and that
   `/personas`, `/gate`, glossary discipline, and the dispatch table are live.

---

## Update

**The target already has a `.claude/` machine. Do NOT do a fresh install or wipe `/.proj/`.**
An update is: *merge the newer machine into `.claude/`*, then *re-index `/.proj/` in place*.

1. **Confirm it's an update.** `$target/.claude/` already exists with the machine tree
   (`agents/`, `skills/`, `hooks/`, `rules/`, `settings.json`). If not, run **Install**.

2. **Clone the machine** into a temp dir:
   ```bash
   target="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
   tmp="$(mktemp -d)"; git clone --depth 1 "<clone-url>" "$tmp"
   ```

3. **Check the version first.** Compare the installed commit against the freshly cloned
   HEAD. If they match, the machine is current — clean up and skip to **Re-index** (the
   project may still have drifted). Only merge when the hashes differ:
   ```bash
   installed="$(cat "$target/.claude/version.log" 2>/dev/null || echo none)"
   latest="$(git -C "$tmp" rev-parse HEAD)"
   if [ "$installed" = "$latest" ]; then
     echo "Machine already up to date ($installed) — skipping merge."; rm -rf "$tmp"
   fi
   ```
   If `version.log` is absent (machine predates stamping), treat it as out of date and merge.
   No `bash`? Read the one-line hash with your file tools and compare it the same way.

4. **Merge the new state over `.claude/`.** Same excludes as install, so instance state
   survives — the nested `.claude/` session dir, `.kern/` store, and lock files are never
   touched. `rsync` without `--delete` overwrites machine files and adds new ones, leaving
   live state in place:
   ```bash
   rsync -a --exclude '.git' --exclude 'README.md' --exclude '.claude' \
            --exclude '.kern' --exclude '*.lock' --exclude '*.lock.json' \
            "$tmp"/ "$target/.claude/"
   ```
   No `rsync`? Copy the same top-level entries with your file tools, overwriting in place. (A
   merge does not *remove* files retired upstream; for that, do a clean reinstall — back up
   `/.proj/` first, it lives outside `.claude/` so it is already safe.)

5. **Re-stamp the version and re-fill the URL.** Record the merged commit, and re-substitute
   the first-line `<url>` in `INSTRUCTIONS.md` with the real raw URL (the merge overwrote it
   with the placeholder):
   ```bash
   git -C "$tmp" rev-parse HEAD > "$target/.claude/version.log"
   ```

6. **Clean up:** `rm -rf "$tmp"`.

7. **Re-index `/.proj/`** — run the **Re-index** section below (reconcile, do not wipe).

8. **Report** which machine files changed under `.claude/`, what you patched in `/.proj/`,
   and confirm `/personas`, `/gate`, glossary discipline, and the dispatch table are live.

---

## Re-index `/.proj/`

Specialize the portable machine to *this* repo. On a fresh install, regenerate from scratch.
On an update, **reconcile in place** — patch only what drifted, never blow away a working
project layer.

### Fresh install — regenerate

1. **Clear `/.proj/` (keep structure, wipe instance state):**
   ```bash
   cd "$(git rev-parse --show-toplevel 2>/dev/null || echo .)"
   mkdir -p .proj/personas .proj/skills
   rm -f .proj/agent.md .proj/project.md .proj/trello.json .proj/improve.json
   rm -f .proj/*.lock .proj/*.lock.json
   rm -f .proj/personas/*.md
   printf 'category,term,definition\n' > .proj/glossary.csv
   ```
   Keep `/.proj/.gitignore` and `/.proj/glossary.md` if present.

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
   - **`/.proj/project.md`** — facts: name, domain (one line), stack, platform, target,
     authoritative spec path, key paths (hot path, entry points, mapping docs), build + test
     + quality-gate commands, CI path.
   - **`/.proj/agent.md`** — the project half of the agent (read by
     `.claude/agents/default.md`): *What this project is*, *Project law* (binding domain
     rules — as hard as machine law), *Domain idioms*, the *persona panel* roster, and
     *build/verify* commands.
   - **`/.proj/glossary.csv`** — seed rows (`category,term,definition`) for ambiguous domain
     terms. Header-only is fine if none are clear yet.
   - **`/.proj/personas/` + `/.proj/personas.md`** — a review panel tuned to the project's
     real risk surfaces. One `*.md` per reviewer (name, role, what they catch);
     `personas.md` indexes them with `**File:** .proj/personas/<name>.md` pointers. If the
     domain is unclear, write an empty `personas.md` stub and tell the user to author it.

### Update — reconcile in place

Re-read the current repo and compare against the existing project layer. Patch only what
drifted; do **not** clear `/.proj/`:
- `project.md` — stack, key paths, build/test/gate commands still accurate?
- `glossary.csv` / `glossary.md` — terms still defined, none renamed away?
- `personas/` — does the updated machine expect persona slots that are missing?
- `agent.md` — identity/domain law still matches the repo's shape?

If `/.proj/` is missing or structurally stale, regenerate it (the fresh-install path above).
If present and largely intact, hand-patch the specific gaps.

**Never touch the machine** (`skills/`, `agents/`, `hooks/`, `settings.json`, `rules/`)
during re-index — it only writes under `/.proj/`.

---

## Install by hand — fallback

No agent? Two steps:

1. **Copy the machine** in as the new repo's `.claude/` (everything but `.git`, the README,
   and `*.lock*`):
   ```bash
   git clone --depth 1 <clone-url> /tmp/machine
   mkdir -p /path/to/newrepo/.claude
   rsync -a --exclude '.git' --exclude 'README.md' --exclude '.claude' \
            --exclude '.kern' --exclude '*.lock*' \
            /tmp/machine/ /path/to/newrepo/.claude/
   git -C /tmp/machine rev-parse HEAD > /path/to/newrepo/.claude/version.log
   rm -rf /tmp/machine
   ```
   `/.proj/` lives outside `.claude/`, so it never tags along. `version.log` stamps the
   installed commit so a later `/oil-me` can detect changes.

2. **Run `/oil-me`.** Open Claude Code in the new repo and run it — it re-indexes `/.proj/`.

---

## Layout

| Path | What |
|------|------|
| `agents/default.md` | the eager-generalist default agent (reads `/.proj`) |
| `agents/*` | dispatch agents (expert-*, manager-*, builder-*) |
| `skills/` | workflow + integration skills (`/gate`, `/personas`, `/improve`, …) |
| `skills/oil-me/` | `/oil-me` — install, update, and re-index `/.proj` |
| `hooks/personas.mjs` | Stop hook — auto-runs the persona panel |
| `hooks/caveman-startup.mjs` | SessionStart hook — primes caveman mode |
| `hooks/statusline.mjs` | statusLine renderer |
| `output-styles/` | the `machine` output style (`outputStyle` in settings) |
| `settings.json` | hook wiring, env, `agent: default` |
| `INSTRUCTIONS.md` | this file — the lifecycle protocol `/oil-me` executes |
| `version.log` | installed machine commit hash (instance state; written on install/update) |
