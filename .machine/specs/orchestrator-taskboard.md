---
id: SPEC-ORCH-001
title: "Orchestrator v2 — delayed taskboard auto-execution"
version: 1.1.1
status: draft
created: 2026-06-14
updated: 2026-06-14
author: machine-manager-spec
priority: high
issue_number: null
---

# Orchestrator v2 — Delayed Taskboard Auto-Execution

## HISTORY

| Version | Date | Author | Change |
|---------|------|--------|--------|
| 1.0.0 | 2026-06-14 | machine-manager-spec | Initial draft. Specifies the v2 ordered taskboard, per-task settle delay, approve-as-override model, ScheduleWakeup scheduler, and the Write permission path. |
| 1.1.0 | 2026-06-14 | machine-builder-skill | Add the board-trust model: TB-013 (driver-owned board, quarantine of untrusted entries) and TB-014 (dispatched agents may not orchestrate or write the board). Closes the control gap where a dispatched agent dropped self-authored entry-files into the live board that could auto-fire without human approval. |
| 1.1.1 | 2026-06-14 | machine-builder-skill | Reconcile TB-010 with shipped code: the orchestrator's `Write` is confined to `/.machine/**` by its `tools:` allowlist (Write, no Edit/NotebookEdit) plus a profile/skill contract, NOT a settings.json deny-rule. A global Write-deny was considered and rejected (deny arrays do not support `!`-negation; the only working deny-except-one-dir form is project-global and would block dispatched specialists from writing project code). TB-013 board-trust is the safety layer. Fixed Scenario 5, integration items, risks, and DoD to match. |

## Context and Relationship to the Existing Skill

This spec EXTENDS and supersedes the orchestration model defined in
`.claude/skills/orchestrate/SKILL.md` and the read-only profile in
`.claude/agents/orchestrator.md`. It is the single source of truth for the v2
*deltas*; it does not restate the existing skill text. Where v1 mechanics are
retained unchanged (validation pipeline of `gate` + `/personas`, the
`SendMessage`-preserves-context redo path, the convergence-toward-empty board,
the attention footer as a rebuilt-every-turn view of disk state), they are
referenced, not copied.

The defining behavioral change from v1 to v2:

- **v1** holds every completed result in a *pending-approval* queue and does
  nothing until the user explicitly approves. Dispatch is immediate; approval is
  the only gate; nothing fires on its own.
- **v2** inverts the *start* gate to a timed default: the driver writes an
  ordered taskboard, each task gets a settle countdown, and a task **auto-fires**
  when its countdown elapses unless the user intervenes. The v1
  post-completion approval gate is RETAINED on top of this — a validated result
  still waits for approval before it is applied or merged.

So v2 has two gates per task: a **pre-fire** gate (timed auto-fire, overridable)
and a **post-completion** gate (explicit approval, inherited from v1).

### Layout decision: one entry-file per task under `/.machine/sessions/`

The board is realized as **one Markdown entry-file per task** under
`/.machine/sessions/`, not as a single monolithic board file.

Justification: the existing system already stores one durable file per subagent
under `/.machine/sessions/` (`<id>.md`), and the `ignite` SessionStart hook
already discovers open work by reading every `*.md` in that directory and parsing
its frontmatter. Reusing that exact pattern means the resume path, the footer
rebuild, and the convergence-toward-empty rule all carry over with minimal new
machinery, and timer-resume requires only adding new frontmatter fields the hook
can read. A single board file would fork the storage model, force whole-file
rewrites on every per-task timer reset (a write-contention and merge hazard), and
break the established per-file lifecycle. The per-task file IS the board entry;
"board order" is a derived ordering over those files (see TB-002), not a separate
artifact.

## Glossary (terms introduced by this spec)

- **taskboard** — the ordered set of per-task entry-files under
  `/.machine/sessions/` that the driver schedules and fires.
- **settle_delay** — the named, configurable countdown applied to a task between
  when it is added and when it auto-fires. Default value: 10 minutes. It is a
  functional scheduling parameter, not an effort estimate.
- **fire_at** — the absolute timestamp at which a settling task becomes eligible
  to auto-fire.
- **settling** — the state of a scheduled task whose `fire_at` has not yet
  passed; its timer is counting down.
- **freeze / frozen** — a user-applied indefinite pause of a task's timer.
- **isolation flag** — a per-task flag marking that the task writes files and
  must run in worktree isolation.

These terms must be reflected into `/.machine/glossary.csv` as a follow-up work
item (see Integration Points), per glossary discipline in machine law.

---

## Requirements (EARS)

### TB-001 — Driver-written ordered taskboard (entry-file per task)

**Ubiquitous.** The orchestrator **shall** persist the taskboard as one Markdown
entry-file per task under `/.machine/sessions/`, where each entry carries at
minimum: a stable `id`, a `label`, an `agent_type`, the job/spawn-prompt, a
`dependencies` list, `added_at`, `fire_at`, `status`, and an `isolation` flag.

**While** a task writes files, the orchestrator **shall** set that task's
`isolation` flag to require worktree isolation.

Acceptance criteria:
- Each task is a separate `<id>.md` file; ids are short and stable (`a1`, `a2`,
  …) consistent with the existing convention.
- Frontmatter of every entry-file contains all minimum fields named above; the
  spawn-prompt body is present and complete enough to execute from alone.
- An entry whose work writes any project file has `isolation` set to require a
  worktree; a read-only/analysis-only task may have it cleared.
- The board has no separate monolithic file; the set of entry-files IS the board.

### TB-002 — Board order is priority

**Ubiquitous.** The orchestrator **shall** treat board order as task priority,
where the explicit ordering of entries (an `order` or sequence field in
frontmatter) determines precedence among otherwise-equally-eligible tasks.

Acceptance criteria:
- Two tasks that become eligible in the same wake are launched in board order.
- Board order is derivable deterministically from the entry-files (it does not
  depend on filesystem listing order); the ordering field is explicit in
  frontmatter.
- Priority orders *precedence among ready tasks*; it does not override
  dependencies or freeze (a higher-priority task still waits on its
  dependencies and is still blocked while frozen).

### TB-003 — Per-task settle countdown with named parameter

**Event-driven.** **When** a task is added to the board, the orchestrator
**shall** set `added_at` to the current time and `fire_at = added_at +
settle_delay`, where `settle_delay` is a named configurable parameter with a
default of 10 minutes.

**Ubiquitous.** The orchestrator **shall** read `settle_delay` from a single
named source of configuration rather than from a hardcoded literal at the point
of use.

Acceptance criteria:
- A newly added task has `status: scheduled`, `added_at` set, and
  `fire_at = added_at + settle_delay`.
- `settle_delay` has a documented name and default (10 minutes) and is
  configurable in one place; changing it changes the computed `fire_at` of
  subsequently added/edited tasks.
- The 10-minute value never appears as an unnamed magic number in the operating
  instructions.

### TB-004 — Editing a settling task resets only its timer

**Event-driven.** **When** the user edits a task that has not yet fired (its job,
label, agent_type, dependencies, or isolation flag), the orchestrator **shall**
reset THAT task by setting `added_at` to the current time and recomputing
`fire_at = added_at + settle_delay`, leaving all other tasks' timers unchanged.

Acceptance criteria:
- The reset trigger is an edit/append to any of the task's content fields (job,
  label, agent_type, dependencies, isolation) while `status: scheduled`.
- After the edit, the edited task's `fire_at` reflects the new `added_at`; sibling
  tasks' `fire_at` values are byte-for-byte unchanged.
- An edit to a task that is already `running`, `pending-approval`, `approved`, or
  `rejected` does NOT reset a timer (there is no pre-fire timer to reset); such a
  request is handled by the redo path, not by edit.
- An edit to a `frozen` task updates content but does not start the timer; the
  task remains frozen (timer resumes only on `approve`/unfreeze semantics per
  TB-007).
- After any reset, the wakeup is re-scheduled if the new `fire_at` changes the
  soonest pending `fire_at` (see TB-008).

### TB-005 — Auto-fire eligibility (timer + dependencies + not frozen)

**State-driven.** **While** a task's `fire_at` has passed AND all tasks named in
its `dependencies` list have reached `approved` AND the task is not frozen, the
orchestrator **shall** consider that task eligible to fire and, on the next
wakeup, launch it.

**Unwanted-behavior.** **If** a task's `fire_at` has passed but one or more of its
dependencies is not yet `approved`, **then** the orchestrator **shall** hold the
task (not launch it) and keep it eligible to fire as soon as the last dependency
becomes `approved`.

Acceptance criteria:
- Dependency satisfaction is defined as: every id in `dependencies` corresponds to
  a task that has reached `approved`. A `rejected` or missing dependency leaves the
  dependent task blocked, and the orchestrator surfaces this in the footer rather
  than firing or silently dropping it.
- A task whose `fire_at` has passed and whose dependencies are all `approved` and
  which is not frozen launches at the next wakeup.
- A frozen task never fires regardless of timer or dependencies.

### TB-006 — Parallel execution with worktree isolation for writers

**State-driven.** **While** more than one eligible task is independent of the
others, the orchestrator **shall** launch them concurrently (in one dispatch
turn), preserving board order only as a tiebreak for precedence.

**Ubiquitous.** The orchestrator **shall** require that every task whose
`isolation` flag is set (i.e. it writes files) run in worktree isolation, so
concurrent file-writing tasks cannot collide on the working tree.

Acceptance criteria:
- Two ready tasks with no dependency relationship between them are dispatched in
  the same turn (`run_in_background: true`), not serialized.
- Each file-writing task's spawn prompt instructs it to operate in its own
  worktree; two concurrent writers never share one working tree.
- A read-only task (isolation cleared) may run without a worktree.
- Dependency edges still serialize where they exist: a dependent task is not
  launched concurrently with a dependency it has not yet seen reach `approved`.

### TB-007 — Approve-as-override and the status lifecycle

**Ubiquitous.** The orchestrator **shall** default to auto-firing each scheduled
task at its `fire_at`, and **shall** provide the user four pre-fire overrides —
`freeze`, `approve`, edit, and `drop` — plus the retained post-completion
approval gate.

The full status lifecycle each task moves through: a task moves from `proposed`
(optional staging) to `scheduled` to `running` to `pending-approval` to
`approved`, or to `rejected` when dropped; and any pre-fire state can move to
`frozen`.

| State | Meaning |
|-------|---------|
| `proposed` | Driver has drafted the entry but it is not yet on the timed board (e.g. mid-clarification). Optional staging state. |
| `scheduled` | On the board with a live `fire_at` countdown (settling). The default state after `add`. |
| `running` | Fired (auto or via `approve`); the background subagent is in flight. |
| `pending-approval` | Subagent finished and was validated (`gate` + `/personas`); awaiting the user's post-completion approval. Inherited from v1. |
| `approved` | User approved the validated result; work is applied/merged; entry-file deleted. Satisfies dependents. |
| `rejected` | User dropped the task; subagent stopped if live; entry-file deleted. |
| `frozen` | Timer paused indefinitely by the user; never fires while frozen. |

Transition rules:

**Event-driven.** **When** the user issues `freeze <id>` on a `scheduled` task,
the orchestrator **shall** set `status: frozen` and stop its countdown
indefinitely (the timer does not advance while frozen).

**Event-driven.** **When** the user issues `approve <id>` on a `scheduled` or
`frozen` task, the orchestrator **shall** fire it immediately (pre-fire override:
launch now, ignoring remaining countdown and freeze) and set `status: running`.

**Event-driven.** **When** the user issues `approve <id>` on a `pending-approval`
task, the orchestrator **shall** apply/merge the validated result, log it, and
delete the entry-file (post-completion gate, inherited from v1).

**Event-driven.** **When** the user edits a `scheduled` task, the orchestrator
**shall** reset its timer per TB-004 (the task stays `scheduled`).

**Event-driven.** **When** the user issues `drop <id>` in any state, the
orchestrator **shall** stop the subagent if live, set `status: rejected`, and
delete the entry-file.

Acceptance criteria:
- `approve` is overloaded by state: pre-fire it means "fire now"; post-completion
  it means "apply the result". Both behaviors are documented in the command table.
- A `frozen` task has a clearly suspended timer: no `fire_at` recomputation occurs
  while frozen, and it is excluded from wakeup scheduling until unfrozen
  (unfreezing is achieved by `approve` to fire now, or by `drop`). Editing a
  `frozen` task updates its content only and leaves it frozen (per TB-004); edit
  is not an unfreeze.
- Reaching `approved` or `rejected` deletes the entry-file; the board converges
  toward empty exactly as in v1.

### TB-008 — In-session ScheduleWakeup scheduler with catch-up

**Ubiquitous.** The orchestrator **shall** maintain a single scheduled wakeup set
to the delta to the soonest pending `fire_at` among non-frozen `scheduled` tasks.

**Event-driven.** **When** the wakeup fires, the orchestrator **shall** launch ALL
tasks that are now due-and-eligible (timer passed, dependencies `approved`, not
frozen) in one turn, then re-schedule the wakeup for the next pending `fire_at`.

**Event-driven.** **When** the board is edited (a task added, edited, frozen,
approved, dropped, or completed), the orchestrator **shall** re-compute the
soonest pending `fire_at` and re-schedule the single wakeup accordingly.

**Unwanted-behavior.** **If** the session ends, **then** no wakeup fires and no
task auto-fires until the next session; the orchestrator **shall not** rely on any
out-of-session timer.

Acceptance criteria:
- At most one wakeup is outstanding at a time; it always targets the soonest
  pending `fire_at`.
- On wake, multiple overdue tasks launch together (catch-up), not one-per-wake.
- After launching due tasks, the wakeup is immediately re-scheduled for the next
  pending `fire_at`; if none remain, no wakeup is scheduled.
- The session-alive limitation is stated explicitly in the skill: auto-fire is an
  in-session behavior only.

### TB-009 — Timer resume on SessionStart

**Event-driven.** **When** a new session starts, the orchestrator **shall**
recompute each task's eligibility from its persisted `fire_at` against the current
time: any task whose `fire_at` is already in the past (and whose dependencies are
`approved` and which is not frozen) is immediately eligible, and the wakeup is
(re)scheduled for the soonest still-future `fire_at`.

Acceptance criteria:
- Tasks whose `fire_at` elapsed while the session was down are eligible at once on
  resume (subject to dependencies and freeze) and launch on the first
  resume-driven evaluation.
- Tasks still in the future have the single wakeup scheduled to the soonest of
  them.
- `frozen` tasks remain frozen across the restart and are excluded from scheduling
  until the user acts.
- Resume reads the entry-files' frontmatter only; it does not require any live
  agent to have survived.

### TB-010 — Board write-path and contract-based confinement

**Ubiquitous.** The orchestrator **shall** be granted `Write` in its `tools:`
allowlist — but NOT `Edit`, `NotebookEdit`, or any kern mutation tool — so it can
create and append taskboard entry-files and its session bookkeeping under
`/.machine/` while remaining unable to modify existing project files in place.
The confinement of that `Write` to `/.machine/**` **shall** be a CONTRACT of the
orchestrator profile (`agents/orchestrator.md`) and the `orchestrate` skill, NOT a
`settings.json` permission rule.

A `settings.json` global `Write`-deny was considered and REJECTED for two verified
reasons: (1) Claude Code permission `deny` arrays do not support `!`-prefixed
gitignore-style negation, so "deny `Write` everywhere, then re-include
`/.machine/**`" cannot be expressed; (2) the only working "deny except one
directory" form (`deny: ["Write"]` plus an allow) is PROJECT-GLOBAL and would block
every dispatched specialist from writing project code, breaking the orchestrator's
dispatch model. `settings.json` permissions therefore remain plain
`{"defaultMode":"bypassPermissions"}` with no `Write` deny, precisely so dispatched
specialists can still write project code.

**Unwanted-behavior.** **If** the orchestrator would otherwise modify an existing
project file, **then** that action is prevented structurally: the profile grants no
`Edit`/`NotebookEdit`, so the driver has no in-place-edit capability, and the
board-trust model (TB-013) is the safety layer against unwanted writes to the
board itself.

Acceptance criteria:
- `agents/orchestrator.md` `tools:` includes `Write` but does NOT include `Edit` or
  `NotebookEdit` (nor any kern mutation tool).
- `settings.json` contains NO `Write` deny-rule; its permissions remain
  `{"defaultMode":"bypassPermissions"}` so dispatched specialists can write project
  code.
- Confinement of the driver's `Write` to `/.machine/**` is contract-based: it is
  stated as a binding rule in both `agents/orchestrator.md` and the `orchestrate`
  skill, not enforced by a harness permission rule.
- TB-013 board-trust is the real safety layer: untrusted entry-files are
  quarantined and never auto-fired, mitigating the absence of a hard harness
  boundary.
- This requirement is recorded as the resolution of the previously unresolved
  "session-file write path" question: the driver now holds a real `Write`
  capability, contract-bound to `/.machine/**` and structurally unable to edit
  existing project files in place.
- Every spawn prompt the board emits still carries machine law + the relevant
  project law and glossary terms, because dispatched subagents — not the driver —
  perform all project writes (this constraint is unchanged from v1 and binding).

### TB-011 — Command surface

**Ubiquitous.** The orchestrator **shall** support the following user commands,
extending the existing v1 command table:

| Command | Action |
|---------|--------|
| `add <desc>` | Create a new task: assign next id, write its entry-file with `status: scheduled`, `added_at: now`, `fire_at: now + settle_delay`, `isolation` set if it writes files; re-schedule the wakeup (TB-008). Reaches `scheduled` after the driver has understood the unit per the existing "understand before dispatch" rule. |
| `edit <id> …` | Amend a `scheduled` (or `frozen`) task's content fields; reset its timer per TB-004; re-schedule the wakeup. |
| `freeze <id>` | Set `status: frozen`; pause its countdown indefinitely; re-schedule the wakeup (TB-007, TB-008). |
| `approve <id>` | Pre-fire: fire the task immediately (override timer/freeze), set `running`. Post-completion: apply/merge the validated result, log it, delete the entry-file. Behavior is selected by current state (TB-007). |
| `drop <id>` | Stop the subagent if live, set `rejected`, delete the entry-file (any state). |
| `show <id>` | Print the full entry: label, agent_type, job/spawn-prompt, dependencies, added_at/fire_at, status, isolation, and (if present) result summary and validation. |
| `redo <id>: <note>` | Retained from v1: `SendMessage` to that task's `agent_id` with the note, set `changes-requested` then `running`; context preserved, never restarts from zero. |
| `adopt <id>` | Move a quarantined `untrusted` entry into the driver's tracked set and schedule it: set `status: scheduled`, `added_at: now`, `fire_at: now + settle_delay`; re-schedule the wakeup (TB-013). The entry then follows the normal lifecycle under the driver's ownership. |

Acceptance criteria:
- Every command above is defined in the v2 skill command table with its exact
  effect and any state-dependence.
- `approve` documents its two state-dependent meanings.
- `redo` retains the v1 `SendMessage`-preserves-context semantics.

### TB-012 — Footer reflects timed board state

**Ubiquitous.** The orchestrator **shall** rebuild the attention footer every turn
from the entry-files on disk, showing each non-terminal task's id, label, status,
and — for `scheduled` tasks — its remaining time until `fire_at`, and — for
`frozen` tasks — a frozen marker.

Acceptance criteria:
- A `scheduled` task shows time-to-fire so the user knows the auto-fire is
  imminent and can override before it fires.
- A `frozen` task is visibly marked as paused.
- The footer never invents an entry not backed by a file; terminal
  (`approved`/`rejected`, deleted) tasks are absent.
- The footer's reply hint lists the v2 commands (`add`, `edit`, `freeze`,
  `approve`, `drop`, `show`, `redo`, `adopt`).

### TB-013 — Driver-owned board and quarantine of untrusted entries

**Ubiquitous.** The orchestrator (the driver in the user-facing session)
**shall** be the only actor permitted to create, schedule, or auto-fire taskboard
entry-files, and **shall** track in-session the set of entry ids it created so it
can distinguish its own entries from any others.

**Unwanted-behavior.** **If** an entry-file appears under `/.machine/sessions/`
that is not in the driver's tracked set (created by a dispatched/sub-agent, left
over stale from a prior session, or produced by a nested cascade), **then** the
orchestrator **shall** treat it as `untrusted`, **shall not** auto-fire it on the
settle timer, **shall** surface it in the footer as needing explicit human review,
and **shall** wait for an explicit user command (`approve`, `drop`, or `adopt`)
before acting on it.

Acceptance criteria:
- The driver maintains an in-session record of the entry ids it created this
  session; an entry id not in that record is `untrusted`.
- An `untrusted` entry is never auto-fired by the scheduler (TB-005/TB-008) and is
  never launched by SessionStart catch-up (TB-009); the settle timer does not
  apply to it.
- The footer lists every `untrusted` entry in the attention section (alongside
  `pending-approval` and blocked tasks) marked as needing human review, never with
  a time-to-fire.
- The driver acts on an `untrusted` entry only after an explicit user command:
  `drop` deletes it; `adopt`/`approve` moves it into the driver's tracked set,
  after which it follows the normal lifecycle. The driver never silently fires,
  applies, or merges an `untrusted` entry.
- On SessionStart resume, any pre-existing entry-file the driver did not create in
  the current session is reconciled as `untrusted` until the user adopts or drops
  it; resume-driven catch-up (TB-009) fires only trusted, driver-created entries.

### TB-014 — Dispatched agents may not orchestrate or write the board

**Ubiquitous.** A dispatched specialist or sub-agent **shall** perform only the
single unit of work in its spawn prompt and report back, and **shall not** enter
orchestrate mode, run `/improve` or any other self-directed autonomous loop on its
own initiative, spawn further work the spawn prompt did not request, or write any
file under `/.machine/sessions/`.

**Unwanted-behavior.** **If** a dispatched agent would otherwise self-direct
beyond its spawn prompt (scope creep, a nested cascade, writing the taskboard),
**then** that action is prohibited; the board-trust model (TB-013) is the backstop
that quarantines any entry-file such an agent nonetheless produces.

Acceptance criteria:
- The prohibition is stated as a hard rule in `orchestrate/SKILL.md` and as a
  one-line consistent note in `agents/default.md` and `agents/orchestrator.md`.
- A dispatched agent does exactly the unit of work in its spawn prompt and reports
  back; it does not invoke orchestrate, `/improve`, or any improvement loop on its
  own initiative, and does not spawn work beyond what the spawn prompt directs.
- A dispatched agent writes no file under `/.machine/sessions/`; only the driver
  authors the board.
- Any entry-file that nonetheless appears from a dispatched agent is caught by
  TB-013 as `untrusted` and is never auto-fired.

---

## Exclusions (What NOT to Build)

- **No out-of-session execution.** v2 does not introduce a daemon, cron, OS
  timer, or background process that fires tasks while no Claude session is alive.
  Auto-fire is strictly in-session; resume on SessionStart is the only
  cross-session mechanism (TB-009).
- **No monolithic board file.** A single combined board document is explicitly
  rejected in favor of the per-task entry-file layout (see TB-001 and the layout
  justification).
- **No new generic worker agent.** v2 routes every task to an existing specialist
  via the dispatch table in `agents/default.md`; it does not create a new
  catch-all worker type.
- **No driver-authored project code.** The added `Write` capability is contract-bound
  to `/.machine/**`, and the driver has no `Edit`/`NotebookEdit` to modify existing
  project files in place; the driver still performs zero project writes (TB-010).
  All project changes go through dispatched, validated, approved subagents.
- **No removal of the post-completion approval gate.** v2 adds a pre-fire timed
  gate; it does not replace or weaken the v1 requirement that a validated result
  waits for explicit approval before being applied/merged.
- **No cross-machine / remote aggregation.** Result collection stays on background
  Agent notifications; no MCP is introduced for remote aggregation (unchanged
  from v1 rationale).
- **No per-task configurable delay (this version).** `settle_delay` is one named
  global parameter with a single default; per-task override of the delay value is
  out of scope for v1.0.0 (freeze/edit cover the manual cases).

---

## Integration Points (work items — not implemented in this spec)

1. **`.claude/skills/orchestrate/SKILL.md`** — rewrite to the v2 lifecycle: add the
   taskboard entry-file schema deltas (`dependencies`, `added_at`, `fire_at`,
   `isolation`, `order`, `proposed`/`scheduled`/`frozen` states), the
   ScheduleWakeup scheduler loop with catch-up and re-schedule-on-edit, the
   settle_delay parameter and its default, the extended command table
   (`add`/`edit`/`freeze`/`approve`/`drop`/`show` + retained `redo`), and the
   footer changes (time-to-fire, frozen marker). Reference v1 retained mechanics;
   do not duplicate them.
2. **`.claude/agents/orchestrator.md`** — add `Write` (but NOT `Edit`/`NotebookEdit`)
   to the `tools:` allowlist and update the "you write nothing — enforced" section
   to "you write only `/.machine/**` — a binding profile/skill contract; you have
   no `Edit`/`NotebookEdit` so you cannot modify existing project files in place",
   reconciling the prose with TB-010. This allowlist change (add `Write`, no `Edit`)
   is the integration point that lets the driver author the board.
3. **`.claude/settings.json`** — no `Write` deny-rule is added (a global deny was
   considered and rejected per TB-010). Permissions remain
   `{"defaultMode":"bypassPermissions"}` so dispatched specialists can still write
   project code. The only settings.json change for v2 is the
   `MACHINE_SETTLE_DELAY_MINUTES` env entry exposing `settle_delay` (TB-003). If
   that uses a settings field not already present, update the Claude Code Version
   Compatibility table in `.claude/rules/coding-standards.md` accordingly.
4. **SessionStart / `ignite` hook** — confirm/extend
   `.claude/hooks/ignite.mjs` so resume reads the new timer fields. The hook
   already parses session-file frontmatter for `id`/`label`/`status`; TB-009
   requires it (or the skill it triggers) to also recompute eligibility from
   `fire_at` vs current time and surface overdue tasks. Decide whether the timer
   recompute lives in the hook (state gathering) or the `ignite` skill (playbook);
   keep the hook non-blocking and error-safe as it is today.
5. **`/.machine/sessions/README.md`** — update to describe the entry as a *task*
   with a timer, not only a finished-subagent record, without duplicating the
   skill schema.
6. **`/.machine/glossary.csv`** — add the terms introduced here (taskboard,
   settle_delay, fire_at, settling, freeze/frozen, isolation flag) per glossary
   discipline.

---

## Plan

### Approach

Land v2 as a documentation-and-config change to the machine payload (skill,
agent, settings, hook, README, glossary). No application code is involved; in
this repo "build" means configuration integrity (hooks pass `node --check`,
`settings.json` parses, all agent/skill references resolve), which is the gate
this change must pass.

### Milestones (priority-ordered, no time estimates)

1. **Write path first (TB-010).** Add `Write` (but not `Edit`/`NotebookEdit`) to the
   orchestrator allowlist and state the contract-based `/.machine/**` confinement in
   the profile and skill; leave `settings.json` permissions as
   `bypassPermissions` (no `Write` deny). Verify `settings.json` parses. This
   unblocks the driver actually writing the board.
2. **Schema + lifecycle (TB-001..TB-007).** Define the entry-file schema deltas,
   the status lifecycle including `frozen`, and the per-task timer-reset rule in
   the skill.
3. **Scheduler (TB-008, TB-009).** Specify the single-wakeup ScheduleWakeup loop,
   catch-up firing, re-schedule-on-edit, and SessionStart resume; wire the hook.
4. **Command surface + footer (TB-011, TB-012).** Extend the command table and the
   footer rendering.
5. **Glossary + README (cleanup).** Reflect new terms and update the sessions
   README.

### Risks

- **Session-death surprise.** Users may expect tasks to fire while away; the
  in-session limitation must be loud in the skill and footer (TB-008, TB-012).
- **Contract not hard-enforced.** The driver's confinement to `/.machine/**` is a
  profile/skill contract, not a harness permission boundary (a global `Write`-deny
  was rejected per TB-010 because it cannot negate one directory and would block
  dispatched specialists). Mitigation: the profile grants no `Edit`/`NotebookEdit`
  (no in-place project edits) and TB-013 board-trust quarantines untrusted writes.
- **Concurrent writer collisions.** If the worktree-isolation rule (TB-006) is not
  honored per task, parallel writers corrupt the tree. The `isolation` flag must
  be set wherever a task writes.
- **Overloaded `approve`.** The two meanings of `approve` (fire-now vs
  apply-result) could confuse users; the footer and command table must make the
  current meaning evident from task state.

---

## Acceptance (Given-When-Then)

### Scenario 1 — Default auto-fire after settle

- **Given** the user runs `add "characterize the auth module"` and the task is
  understood and written with `status: scheduled`, `fire_at = added_at +
  settle_delay`,
- **When** `settle_delay` elapses with the session alive and no dependencies and
  no override,
- **Then** the wakeup fires, the task launches to its specialist with a complete
  spawn prompt (machine law + project law + glossary terms), and its status
  becomes `running`.

### Scenario 2 — Edit resets only that task's timer

- **Given** tasks `a1` and `a2` are both `scheduled` with distinct `fire_at`
  values,
- **When** the user edits `a1`'s job before it fires,
- **Then** `a1`'s `added_at` and `fire_at` are recomputed from now, `a2`'s timer
  is unchanged, and the single wakeup is re-scheduled to the new soonest
  `fire_at`.

### Scenario 3 — Freeze then approve-as-override

- **Given** task `a3` is `scheduled` and the user runs `freeze a3`,
- **When** the user later runs `approve a3` while it is `frozen`,
- **Then** `a3` fires immediately (override), becoming `running`, despite having
  been frozen and regardless of remaining countdown.

### Scenario 4 — Dependency gating with catch-up

- **Given** `b2` depends on `b1`, both overdue on session resume, `b1` not yet
  `approved`,
- **When** the session starts and timers are recomputed (TB-009),
- **Then** `b1` is eligible and launches, `b2` is held until `b1` reaches
  `approved`; when `b1` is approved, `b2` becomes eligible and launches at the
  next evaluation.

### Scenario 5 — Write containment (contract-based)

- **Given** the orchestrator has `Write` but no `Edit`/`NotebookEdit` in its
  allowlist, `settings.json` carries no `Write` deny-rule, and the profile/skill
  contract confines the driver's writes to `/.machine/**`,
- **When** the driver needs to record board state and, separately, a project source
  file must change,
- **Then** the driver writes the taskboard entry-file under `/.machine/sessions/`
  successfully; it does not attempt an in-place edit of any existing project file
  (it has no `Edit` capability) and honors the contract by not authoring project
  code, so the project change flows instead through a dispatched specialist that
  writes the project file in its own (worktree-isolated) context.

### Edge cases

- A dependency is `rejected`: the dependent task is surfaced as blocked in the
  footer, not silently fired or dropped.
- The session ends mid-countdown: nothing fires; on resume, overdue tasks are
  immediately eligible and future tasks re-scheduled (TB-009).
- Two independent tasks fire on the same wake: both launch concurrently; both
  writers use worktree isolation (TB-006).
- `edit` on a `running`/`pending-approval` task: not a timer reset — routed to
  `redo` (TB-004).
- An entry-file the driver did not create appears under `/.machine/sessions/` (a
  dispatched agent wrote it, or it is a stale leftover): it is `untrusted`, never
  auto-fires, and is surfaced in the footer for human review until adopted or
  dropped (TB-013).
- A dispatched agent attempts to run `/improve` or spawn nested work beyond its
  spawn prompt: prohibited (TB-014); any entry-file it nonetheless drops is caught
  as `untrusted` (TB-013).

### Quality gate

- `settings.json` parses as valid JSON and contains NO `Write` deny-rule
  (permissions remain `{"defaultMode":"bypassPermissions"}`).
- `agents/orchestrator.md` and `orchestrate/SKILL.md` reference only resolvable
  agents/skills.
- Any modified hook passes `node --check`.
- English-only, no emoji, no code examples for conceptual explanations, no
  time/duration estimates (the named `settle_delay` parameter is permitted).

### Definition of Done

- All fourteen requirements (TB-001..TB-014) are implemented across the
  integration points, each with its acceptance criteria met.
- The six locked decisions are each traceable to a requirement:
  ordered driver-written board → TB-001/TB-002; per-task settle countdown →
  TB-003/TB-004; parallel-where-independent → TB-005/TB-006; approve-as-override +
  lifecycle → TB-007; ScheduleWakeup in-session scheduler + resume →
  TB-008/TB-009; write-path/permissions → TB-010.
- v1 retained behaviors (post-completion approval, redo-preserves-context,
  validation pipeline, convergence-to-empty) are intact.
- The glossary and sessions README are updated; the change passes the
  configuration-integrity gate.
