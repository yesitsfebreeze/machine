---
name: mine
description: Mine the addon graph for this repo — survey the `mine/` kit (the graph of unregistered agents, skills, and hooks) and the current repo, match the best-fit tools, slot them in (copy + register in plugin.json / hooks.json), wire any prerequisites, and record the decision in kern so each session compounds. Run it a few times and the machine flows like butter. Trigger: "/mine", "mine the graph", "mine this repo", "slot the right tools", "what should we slot".
---

# /mine — slot the best-fit addons into this repo

The machine ships bare-bones. The good stuff lives unregistered in the **mine kit**
(the machine plugin payload — exact path in `/.machine/ENV.md`, see *Where the mine
graph lives* below) — the **mine graph**: extracted agents, skills, and hooks nothing
loads by default. `/mine` realizes the "/oil integration (intended)"
note in the kit's `README.md` as its own skill: it reads the graph, reads *this*
repo, and slots in the tools that fit — incrementally and idempotently. Run it
across a few sessions and the active toolset converges on exactly what this repo
needs.

## Where the mine graph lives — resolve this first

The kit ships inside the machine **plugin payload**, not the target project. Every
`$MINE/…` path below is the kit root, which you resolve from `/.machine/ENV.md` — the
env cache the SessionStart hook writes each session with the install-correct absolute
paths. Do **not** guess it from the project CWD or a git root: when the machine is
installed as a plugin, neither contains `mine/`.

```bash
source .machine/ENV.md   # sets MACHINE_PLUGIN_ROOT / MACHINE_PROJECT_DIR / MACHINE_MINE / MACHINE_MESH
MINE="$MACHINE_MINE"
```

Remote source: `github.com/yesitsfebreeze/machine` under `/mine/`. Slot targets
(`.claude/`, `.claude-plugin/plugin.json`) are the **machine** side — under
`$MACHINE_PLUGIN_ROOT`, never the target project's CWD.

`/mine` writes the **machine** side (registers addons in `.claude/` + manifests).
`/oil` owns the **project layer** (`/.machine/`). They are siblings, not rivals:
`/oil` specializes; `/mine` equips. `/oil` fires `/mine` automatically once its
re-index is done, so a freshly-oiled repo is also freshly-equipped — but `/mine`
runs standalone too, whenever you want to re-survey the graph without re-indexing.

## Phase 1 — survey the mine graph

Catalog what is available to slot. Do **not** read whole files — read the
frontmatter (`name` + `description`) only:

1. Read `$MINE/README.md` for the current inventory and the slotting protocol.
2. Collect each candidate's `name` + `description`:
   - `$MINE/agents/*.md` — frontmatter `name` + `description`.
   - `$MINE/skills/*/SKILL.md` — frontmatter `name` + `description`.
   - `$MINE/hooks/*.mjs` — the leading comment / purpose.

Use the Explore agent or `ctx_*` if the graph is large, so only the catalog
(not raw bodies) enters context.

## Phase 2 — survey the repo

Understand what this repo *needs* and what is *already slotted* (idempotency):

1. **Repo identity:** read `/.machine/project.md` + `/.machine/agent.md` for stack,
   domain, platform, and the persona panel's risk surfaces. If the project layer is
   missing, tell the user to run `/oil` first, or scan the repo directly
   (manifests, entry points, CI) for the same facts.
2. **Already slotted:** read `.claude-plugin/plugin.json` (`agents` / `skills` arrays)
   and `.claude/hooks/hooks.json`. Anything registered there is done — never re-slot it.
3. **Prior decisions:** `mcp__kern__query` for past `/mine` sessions — what was slotted,
   what the user **rejected** and why. Never re-propose a rejected addon without a
   changed reason; surface the prior decision instead.

## Phase 3 — match

Score each *unslotted* candidate against this repo's stack and risk surfaces.
Slot a tool only when the repo has the work it serves — match capability to
evidence, not to vibe:

- A `expert-frontend` agent fits a repo with a UI; an `expert-security` agent fits
  a repo with an auth/network boundary; `tool-ast-grep` fits a large polyglot
  codebase; `perf-gate` fits a repo with a perf budget in its project law.
- Prefer **a few high-confidence picks per session** over a big speculative dump.
  Butter comes from compounding, not from one flood. A couple sessions, a couple
  picks each.
- For every pick, name the evidence (the file, stack fact, or persona concern that
  justifies it) and the prerequisite cost (an MCP server, a CLI binary, an API key).

Present a ranked shortlist: addon, one-line rationale tied to repo evidence, and
prerequisite cost. Skip anything already slotted or previously rejected.

## Phase 4 — confirm

Ask the user which of the shortlist to slot (default to the top high-confidence
picks). Nothing installs without the go-ahead. If the user rejects one, capture the
reason for Phase 6.

## Phase 5 — slot it in

For each approved addon, follow `$MINE/README.md`'s protocol exactly (the `.claude/`
and `.claude-plugin/` slot targets are the machine plugin's own payload — the same
root `$MINE` came from — not the target project's CWD):

1. **Agent:** copy `$MINE/agents/<name>.md` → `.claude/agents/<name>.md`. Add
   `"./.claude/agents/<name>.md"` to the `agents` array in `.claude-plugin/plugin.json`.
   **Sanitize on slot:** strip any `permissionMode: bypassPermissions` line from the
   copied frontmatter — the machine deliberately keeps `bypassPermissions` out of
   installs (see `settings.json`). An agent that needs it is a red flag to raise with
   the user, never a silent slot.
2. **Skill:** copy the folder `$MINE/skills/<name>/` → `.claude/skills/<name>/`. Add
   `"./.claude/skills/<name>"` to the `skills` array in `.claude-plugin/plugin.json`.
3. **Hook:** copy the script into `.claude/hooks/` and restore its entry in
   `.claude/hooks/hooks.json`.
4. **Prerequisites:** wire anything the addon needs — an MCP server in plugin.json
   `mcpServers`, a CLI binary (tell the user the install command, e.g. `sg` for
   ast-grep, the Codex CLI for codex-peer-review), or an API key env var. State
   clearly what the user must install by hand.

Keep one clean implementation: a slotted addon is **moved into service**, not
duplicated — leave the `mine/` copy as the kit's source of truth, and never let an
addon's `name:` collide with a registered one.

Then run `/gate` to confirm config integrity (manifest parses, agent names unique,
skill `name:` matches dir, every referenced skill resolves).

## Phase 6 — record and report

1. **Ingest the decision into kern** (`mcp__kern__ingest`) so the next `/mine`
   session compounds instead of re-deciding: one excerpt per session with a stable
   `object_id` (e.g. `mine-session-<repo>`), titled, listing what was slotted (with
   evidence), what was rejected (with reason), and what prerequisites remain
   pending. Skip silently if kern is down.
2. **Report:** what got slotted, what the user must install by hand, the gate result,
   and the top one or two candidates left for the next session. State plainly how
   close the toolset is to "buttered" for this repo.

## Boundaries

- `/mine` writes the **machine** side only: `.claude/agents/`, `.claude/skills/`,
  `.claude/hooks/`, and the `.claude-plugin/plugin.json` / `hooks.json` manifests.
  Never touch `/.machine/` — that is `/oil`'s job.
- Never edit a `mine/` source file; only copy out of it.
- Never slot speculatively. No evidence in the repo → no slot.
- After registering agents or skills, the user must reload the plugin
  (`/plugin`) for Claude Code to pick them up; say so in the report.
