# Flow — how the machine works

This document describes the machine end to end: what it is, how it is structured,
how it gets into a repo, and how a single request flows from an idea to merged code.
It is the map; the canonical truth lives in `.claude-plugin/plugin.json`,
`.claude/agents/default.md` (machine law), and `.claude/skills/oil/SKILL.md`.

---

## 1. The core idea — two layers

The machine is split into two halves that never mix:

```
the machine plugin   the portable payload  (.claude/) — installed + updated via /plugin
/.machine/              the project layer     — written per repo by /oil, never shipped
```

- **The portable payload** (`.claude/`) is project-agnostic: a roster of agents,
  on-demand skills, lifecycle hooks, rules, output styles, and bundled MCP servers.
  It is identical in every repo.
- **The project layer** (`/.machine/`) is the per-repo brain: identity, hard laws,
  stack facts, vocabulary, and the review panel. `/oil` reads the repo and writes it.

One portable machine, one per-repo brain. The same plugin behaves like a Rust expert
in one repo and a frontend expert in the next, because only the project layer changes.

---

## 2. Getting in — install and oil

```
/plugin marketplace add yesitsfebreeze/machine
/plugin install machine@machine
```

Installing namespaces every component under `machine:` (`machine:gate`, etc.) and
registers the agents, skills, hooks, and MCP servers from `plugin.json`.

Then `/drill` (or `/oil`) brings the repo up:

1. **Bootstrap (cold repo only).** Installs the companion daemons and prerequisites —
   `kern` (memory), the `hub` daemon (coordination + board), the `git-fs` plugin, and
   the bundled MCP servers. Same idempotent pass as `just bootstrap` /
   `bash scripts/bootstrap.sh`.
2. **Oil.** `/oil` reads the codebase and writes the project layer under `/.machine/`:
   `agent.md` (identity + laws), `project.md` (stack, key paths), `glossary.csv`,
   `personas/` (review panel), and `graph.json` (a generated index of every agent,
   skill, hook, and the `mine/` kit).
3. **Mine.** `/oil` then fires `/mine`, which surveys the `mine/` addon kit against the
   repo and suggests best-fit specialists to slot in.

Updates are the plugin system's job (`/plugin update machine`); `/oil` re-indexes the
project layer whenever the repo changes shape.

---

## 3. Who does the work — the agent core

Four agents stay loaded; everything else is slotted on demand.

| Agent | Role |
|-------|------|
| `default` | Eager-generalist driver — whole toolbelt, bias-to-verify. Drives most work itself, routes domain decisions to specialists. |
| `drill` | Session driver / orchestrator. Grills a request into a shape, then runs it autonomously to a green build, surfacing only to land it. |
| `manager-tdd` | Test-driven implementation (RED-GREEN-REFACTOR) for greenfield work. |
| `manager-ddd` | Domain-driven implementation (ANALYZE-PRESERVE-IMPROVE) for legacy code. |

The rest of the toolbelt — the `expert-*` and `manager-*` specialists, extra skills,
extra hooks — lives in `mine/` at the repo root. Nothing there is callable until it is
copied into `.claude/` and registered in `plugin.json` (`/mine` does this). The graph
in `/.machine/graph.json` lists what exists and what is still an unregistered orphan.

---

## 4. The main loop — how a request flows

There are two entry shapes depending on how concrete the request is.

### 4a. Exploratory — Brainstorm mode

A fuzzy prompt ("what if", "I'm thinking about", no concrete action verb) enters
**Brainstorm mode**: a conversation, no file writes, no dispatch. The driver tracks
three things — **WHAT** (the problem), **HOW** (rough direction), **WHY NOW** (worth
doing). When all three are non-vague, the idea has *crystallized* into a dispatchable
task. `/promote` can then turn crystallized findings into board tickets.

### 4b. Concrete — the drill (grill-first, then smooth)

A concrete job runs the drill flow. The drill owns the conversation; a background
subagent owns each unit of real work. The user agrees the shape **once** in the grill;
from there the job runs autonomously to a green build. The drill stops for the user at
exactly one point in the happy path — landing into `main` — and otherwise only on a
blocker it cannot resolve.

```
   grill ──(user agrees the shape)──▶ plan agent (markdown brief, stored)
                                                       │
                                                       ▼  (auto)
   implement (miner, gitfs branch) ──▶ gate until green ──▶ review (personas + codex, advisory)
                                                       │
                                            SURFACE: land to main?   ← the one stop
                                                       │
                                                       ▼
                                       3-way merge ──▶ close
```

1. **Grill.** Refine with the user one question at a time (each question carries a
   recommended answer; explore the codebase instead of asking when it can answer)
   until *the user* calls the shape valid. No writes, no dispatch yet. The user's
   agreement is the single go-ahead the whole job needs.
2. **Plan agent (auto).** One subagent writes the implementation plan as a plain
   Markdown brief — a what-and-how the miner builds from, not code. The plan is
   supersede-aware: if it replaces an existing implementation, it rips the old one out
   in the same change (machine law: one clean implementation). The brief is stored to
   `.machine/plans/<id>.md`.
3. **Implementation agent (a miner, auto).** Builds on its own `gitfs/<id>` branch
   *inside the orchestrator's worktree*, reads the stored brief, writes real code, and
   runs the `gate` skill until the build is green.
4. **Review (advisory, consolidated).** On green, one review pass runs against the
   diff — the `personas` panel plus `codex-review` when codex is present — scaled to
   the size of the change. Advisory only: it informs the user, it never gates.
5. **Surface to land.** The drill presents the diff, the gate result, and the review
   synthesis, and *proposes* a merge into `main`. This is the one stop in the happy
   path. Nothing merges without approval.
6. **Land and close.** On approval, the drill takes the `branch:main` lock, 3-way
   merges the branch into `main` with `git_fs_merge`, releases the claim, prunes the
   branch, and clears the job from the roster.

Between the user's agreement and the landing proposal, the flow is smooth: plan,
implement, gate, and review need no further prompt. The drill pulls the user back in
early only on a blocker it cannot resolve — a gate the miner cannot turn green, a merge
conflict, or a question the spawn prompt did not cover (routed through the
`questioneer`). The only hard blocker for a merge is a green build plus the user's
approval; personas and codex are advisory throughout.

---

## 5. The roster and worktree topology

**The roster lives in the hub.** There is no markdown ledger and no
`/.machine/sessions/` directory. The hub's live claims and roster
(`mcp__hub__roster`, `mcp__hub__claims`) ARE the list of running agents; the drill
rebuilds a footer from them every turn and projects each job onto a board card. A
landed or dropped job releases its claim and clears from the footer — there is no
file to delete and no "board trust" quarantine to police. Per-job durable artifacts
live where they belong: the brief in `.machine/plans/<id>.md`, the work on the
`gitfs/<id>` branch, and the goal / stage posts / report in the hub.

**Main is always free, one worktree per orchestrator.** The orchestrator never edits
or checks out the human's `main` working tree. Each orchestrator gets one worktree
under `/.machine/worktrees/`, and builds every feature it drives inside that one
worktree as a git-fs branch:

```
repo main checkout (the human's)              ← LEFT FREE, never touched
/.machine/worktrees/
  drill-<sid>/   ← THIS orchestrator's one worktree, branch drill/<sid> off main.
                   Every feature it drives is a git-fs branch here:
                     gitfs/<a1>, gitfs/<a2>, ...   (one miner per branch)
  drill-<sid2>/  ← a second orchestrator, fully isolated in its own worktree
```

Each miner edits through **git-fs** (per-edit commits on its own `gitfs/<id>` branch),
so concurrent miners never collide on the working tree even though they share one
physical worktree — git-fs gives each branch a virtual filesystem. Isolation is at the
orchestrator level: two drivers running at once each get their own `drill-<sid>`
worktree. Implementation reaches `main` only through an approved 3-way `git_fs_merge`,
serialized by the `branch:main` claim so concurrent drivers contend one landing at a
time.

---

## 6. PSAIDO — shelved

**PSAIDO is currently disabled.** It was an experiment in writing plans as a
pseudo-code language for LLMs to read (`docs/psaido.md`). The translation hop — plan
agent writes psaido, miner translates psaido into code — added a lossy intermediate
step, so the flow now uses a plain Markdown brief that the miner reads directly. The
`docs/psaido.md` spec is kept for reference but is not part of the active flow; the
plan agent writes prose-and-structure, not psaido.

---

## 7. The supporting systems

These daemons and servers give the fleet memory, coordination, and isolation.

| System | Kind | Role |
|--------|------|------|
| `kern` | companion plugin | Per-directory memory daemon. Auto-captures decisions, recalls them per prompt and at session start. Query it before re-deciding anything (`mcp__kern__query`); ingest durable knowledge (`mcp__kern__ingest`). |
| `git-fs` | companion plugin | Per-session virtual git filesystem. Each feature is a `gitfs/<id>` branch; every edit is a commit; merges are explicit. N agents, 1 worktree, 0 collisions. |
| `hub` | bundled MCP daemon | Fleet coordination AND the live roster. Live roster, atomic cross-process claims/leases (claim a feature before building so two agents never build the same thing), and durable mail. Verbs: `register`, `roster`, `claim`, `claims`, `release`, `post`, `inbox`, `read`. |
| `board` | bundled MCP (hub) | Kanban board at http://localhost:7777 — the roster's visual surface, rendered from the same hub state as the footer. One card per job. Tickets enter via `/promote`. |
| `context7` | bundled MCP | Current versioned library/framework docs — use before guessing an API. |
| `pdf-reader` | bundled MCP | PDF inspect, search, render, OCR, region extraction. |
| `context-mode` | bundled MCP | Keeps large output out of context via `ctx_*` tools — process logs/tests/git output without dumping bytes into context. |

---

## 8. Quality and review

- **`gate`** — the language-agnostic quality gate. Format, lint, tests, and build in
  one pass-fail report. Run before any commit. For this repo it runs the parse +
  dispatch-integrity + standards checks.
- **`personas`** — the data-driven review panel (defined in `/.machine/personas/`).
  One reviewer per lens runs in parallel, then a synthesis pass returns a ship verdict.
  Run after any non-trivial change.
- **`codex-review`** — advisory second-AI review via the Codex CLI at the drill's
  single review point (the consolidated review of the finished diff, with personas).
  Never gates; skips cleanly if codex is absent.
- **`questioneer`** — the single ongoing chat that resolves questions parallel agents
  raise. Agents `post` blocking questions to `topic:questions` and wait; the
  questioneer aggregates, presents each to the user, and writes the answer back.

The review is consolidated into one advisory pass on the finished diff, scaled to the
size of the change — a small change gets a light pass, a large or structural one gets
the full panel. It informs the landing decision; it never gates.

---

## 9. The laws that bind every step

Machine law (in `.claude/agents/default.md`) applies in every repo:

- **Root cause, never a patch.** Fix the actual cause.
- **One clean implementation.** Remove obsolete code in the same commit; no parallel
  duplicates.
- **Glossary discipline.** Check `/.machine/glossary.csv` before using an ambiguous
  term; update it on any correction or rename.
- **Memory lives in kern.** Query before re-deciding; ingest durable knowledge.
- **Track work on the board.** One card per task, column = status, assignee = owner.
- **Serialize writes to shared `main`.** Hold the `branch:main` claim before
  editing/merging the shared tree.
- **File machine defects.** A broken tool/hook/daemon gets a `/report` before you work
  around it.

Project law (in `/.machine/agent.md`) is just as binding and is written per repo by
`/oil` — for this repo: English-only instructions, no external product naming, no
emoji in instruction docs, single source of truth, dispatch integrity, and verify by
parse (there is nothing to run).

---

## 10. The whole picture in one breath

Install the plugin → `/drill` bootstraps deps and `/oil` writes the project layer →
the generalist drives, recalling from kern and gating before commits. For a real
feature the drill grills it into a shape; once you agree, it runs on its own — a plan
agent writes a Markdown brief, a miner builds it on a git-fs branch inside the
orchestrator's one worktree until the gate is green, and one advisory review (personas
+ codex) runs on the diff. The drill then proposes a merge; on your one yes it 3-way
merges into `main` and clears the job. One landing decision, no timers, main always
free, the roster in the hub, everything ends up in git-fs.
