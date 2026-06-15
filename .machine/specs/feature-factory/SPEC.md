---
id: SPEC-FACTORY-001
title: "Feature factory — the repo as a fleet of dispatchable senior programmers"
version: 1.3.0
status: accepted
created: 2026-06-15
updated: 2026-06-15
author: machine-default (loop @target, iteration 2)
priority: high
issue_number: null
---

# Feature Factory

A coordination layer that turns the machine into a fleet of dispatchable senior
programmers. Each job hands one agent a full eight-stage lifecycle (concept ->
plan -> implement -> test -> persona analysis -> evaluate -> fix -> present/close),
runs it inside a subagent we can communicate with, isolates it on its own
`git-fs` branch, and uses `mesh` so two agents never build the same feature twice.

This document specifies WHAT and WHY in EARS form. It prescribes behaviour and
contracts, not function names, prompt wording, or file layouts beyond the schema
the ledger requires. It builds on `SPEC-COMM-001` (mesh) and the existing
`orchestrate`, `personas`, and `gate` machinery.

## HISTORY

| Version | Date | Author | Change |
|---------|------|--------|--------|
| 1.3.0 | 2026-06-15 | machine-default | Stage 8 present-and-close: CLOSE-FACTORY-001 written mapping all 8 acceptance criteria to evidence (AC4 live-verified, AC7 release-verified, rest wired); added feature-factory/README.md doc index; status draft -> accepted. |
| 1.2.0 | 2026-06-15 | machine-default | Stage-2 plan written (PLAN-FACTORY-001). Resolved D3 (stand down + register interest on held claim), D4 (three-iteration fix-loop bound then escalate), D5 (mesh = durable state/coordination, SendMessage = live operator steering). Amended R9: factory agent posts stage to mesh and never writes the ledger; the driver is sole ledger writer and reconciles mesh->ledger, resolving the conflict with SPEC-ORCH-001 TB-013/TB-014. |
| 1.1.0 | 2026-06-15 | machine-default | Added D6 (RESOLVED): the lifecycle runner may be a dispatched factory agent, not only the main-loop driver, per target.md's communicable-subagent requirement; default.md now distinguishes stage dispatch from factory-job dispatch. |
| 1.0.0 | 2026-06-15 | machine-default | Initial draft. Formalizes G1 (lifecycle runner = the default agent), G2 (feature ledger = the orchestrate taskboard extended with stage/branch/claim_id), and G3 (the dedup handshake) from CONCEPT-FACTORY-001 into EARS requirements with numbered acceptance criteria. Carries forward D1/D2 (resolved) and D3/D4 (open). |

---

## 1. Problem statement

### 1.1 The target

From `target.md`: the repo should behave like a real senior programmer we hand
jobs to -- a generalist, not a domain specialist. Given a job, that programmer
runs a full lifecycle end to end (the eight stages above), and the whole
lifecycle runs inside a subagent we can communicate with. We dispatch several
jobs at once, isolate each on its own `git-fs` branch, track open features, and
let the agents talk so two of them never build the same thing twice.

### 1.2 What already exists

Per `CONCEPT-FACTORY-001` section 2, every primitive exists: `kern` (memory),
`mesh` (awareness/claims/chat), `git-fs` (per-job isolation + merge-on-stop), the
`orchestrate` taskboard (async dispatch + settle queue + approval gate), and one
specialist per lifecycle stage. The factory is wiring, not new infrastructure.

### 1.3 The gap

No single dispatchable unit chains the eight stages, maps a feature to its claim
and branch, and refuses to start work a peer already holds. Three pieces close
the gap:

- **G1 -- the lifecycle runner.** The unit that walks all eight stages itself,
  delegating one stage to a specialist only for depth and gating each handoff.
- **G2 -- the feature ledger.** The single record mapping each feature to its
  lifecycle stage, its `git-fs` branch, and its `mesh` claim.
- **G3 -- the dedup handshake.** The startup ritual every factory agent runs
  before stage 1 to guarantee at most one agent owns a feature at a time.

## 2. Resolved decisions

- **D1 (RESOLVED) -- the runner is the `default` agent.** No separate `factory`
  skill or agent type. The eight-stage loop is baked into
  `.claude/agents/default.md` ("The job lifecycle"). G1 is agent behaviour, not a
  new artifact.
- **D2 (RESOLVED) -- the ledger is the orchestrate taskboard.** Reuse the
  per-task entry-files under `/.machine/sessions/`, extended with three fields:
  `stage` (lifecycle position, orthogonal to orchestration `status`), `branch`
  (`git-fs` `agent/<id>`), and `claim_id` (mesh claim handle). No parallel
  structure, no new directory.
- **D6 (RESOLVED) -- the runner may be a dispatched factory agent, not only the
  driver.** `target.md` requires the lifecycle to run "inside a subagent we can
  communicate with". The default agent therefore has two dispatch roles: a *stage
  dispatch* (do one unit, report back) and a *factory-job dispatch* (own one
  feature's full eight-stage lifecycle on its own branch, pulling in
  stage-specialists, coordinating via mesh). Owning one lifecycle is NOT
  orchestrating -- a factory agent never runs the orchestrate taskboard, never
  manages an approval queue, and never spawns further factory agents -- so the
  binding "dispatched agents never orchestrate" law still holds. Baked into
  `.claude/agents/default.md` (role section + "The job lifecycle"). Strengthens R1.

## 3. Definitions

| Term | Meaning |
|---|---|
| **job** | A one-line unit of work handed to a factory agent. |
| **factory agent** | A `default`-agent session running the eight-stage lifecycle for one job. |
| **feature** | The named thing a job builds; the unit a `mesh` claim locks. |
| **ledger entry** | A taskboard entry-file under `/.machine/sessions/` carrying `stage`, `branch`, `claim_id`. |
| **stage** | One of the eight lifecycle positions (concept ... present/close). |
| **handshake** | The G3 pre-stage-1 ritual: `roster` + `claims` + `claim` + intent `post`. |

## 4. Requirements (EARS)

### 4.1 G1 -- the lifecycle runner

- **R1 (ubiquitous).** A factory agent SHALL drive a job through eight ordered
  stages: (1) concept, (2) plan, (3) implement, (4) test, (5) persona analysis,
  (6) evaluate, (7) fix, (8) present and close.
- **R2 (state-driven).** WHILE a stage is active, the agent SHALL own that stage
  itself and delegate to a specialist ONLY when the stage needs depth it would
  otherwise guess at; the delegated specialist SHALL return to the agent, which
  owns the next handoff.
- **R3 (event-driven).** WHEN a stage completes, the agent SHALL post progress to
  `mesh` recording the new stage before beginning the next.
- **R4 (event-driven).** WHEN stages 4-6 (test, persona analysis, evaluate)
  surface required changes, the agent SHALL perform stage 7 (fix) and re-run
  stages 4-6, looping until the persona panel returns a ship verdict OR the
  iteration bound (D4) is reached.
- **R5 (event-driven).** WHEN stage 8 begins, the agent SHALL present the result
  into the orchestrate approval queue and merge via `git-fs`; it SHALL NOT merge
  to `main` before the gate passes.
- **R6 (unwanted behaviour).** IF the `gate` skill fails at any point in stages
  4-8, THEN the agent SHALL NOT advance to present/close and SHALL return to
  stage 7.

### 4.2 G2 -- the feature ledger

- **R7 (ubiquitous).** The factory SHALL use the orchestrate taskboard under
  `/.machine/sessions/` as the single feature ledger; it SHALL NOT create a
  parallel feature-tracking structure.
- **R8 (ubiquitous).** Each ledger entry SHALL carry `stage`, `branch`, and
  `claim_id` in addition to the existing orchestration fields, where `stage` is
  orthogonal to orchestration `status`.
- **R9 (event-driven, amended v1.2.0).** WHEN a factory agent crosses a stage
  boundary, it SHALL `post` the new stage to `mesh` (R3) and SHALL NOT write the
  ledger itself. The driver, as sole ledger writer (SPEC-ORCH-001 TB-013/TB-014),
  SHALL reconcile that mesh post into the entry's `stage` field on its next turn.
  Rationale: a factory agent is a dispatched agent; TB-014 forbids dispatched
  agents writing `/.machine/sessions/` and TB-013 quarantines any entry they
  write. mesh is the source of truth for stage; the ledger is the driver's durable
  projection. See PLAN-FACTORY-001 section 1.
- **R10 (event-driven).** WHEN a factory agent acquires its branch and claim, it
  SHALL record `branch` and `claim_id` on the ledger entry before stage 1 begins.

### 4.3 G3 -- the dedup handshake

- **R11 (ubiquitous).** BEFORE stage 1, a factory agent SHALL run the handshake:
  query `mesh` `roster` and `claims`, attempt `claim` on the feature, and `post`
  an intent broadcast.
- **R12 (unwanted behaviour).** IF the feature's claim is already held by a live
  peer, THEN the agent SHALL NOT begin stage 1 and SHALL stand down or queue per
  D3, leaving no ledger entry in an active stage.
- **R13 (event-driven).** WHEN a factory agent closes a job (stage 8 accepted) OR
  abandons it, it SHALL `release` the `mesh` claim and mark the ledger entry
  closed.
- **R14 (state-driven).** WHILE a factory agent holds a claim, it SHALL keep its
  `mesh` presence fresh so peers can distinguish a held-and-alive claim from a
  stale one (per `SPEC-COMM-001` staleness/expiry).

## 5. Acceptance criteria

1. Given a one-line job and a clean claim, a `default`-agent session walks all
   eight stages without an operator hand-driving each handoff.
2. Each stage transition is observable in `mesh` (R3) and reflected in the
   ledger entry's `stage` field (R9), and the two never disagree.
3. A ledger entry for an active job always resolves to exactly one `branch` and
   one `claim_id` (R8, R10).
4. Two factory agents handed the same feature result in exactly one doing the
   work; the second stands down or queues (R11, R12) and creates no competing
   branch.
5. The gate must pass before any factory branch merges to `main` (R5, R6).
6. Stages 4-6 loop into stage 7 and back until a ship verdict or the D4 bound
   (R4); a non-shipping panel result never reaches present/close.
7. Closing or abandoning a job releases the claim and closes the ledger entry
   (R13); no orphan claim survives a finished job.
8. No parallel feature-tracking structure exists outside `/.machine/sessions/`
   (R7).

## 6. Decisions (resolved in PLAN-FACTORY-001)

- **D3 (RESOLVED) -- lost-the-claim: stand down + register interest.** On a held
  claim the agent does not begin stage 1; it `post`s a deferred-interest note to
  `mesh` and exits, leaving no active ledger entry. No automatic takeover in v1.
  Shapes R12.
- **D4 (RESOLVED) -- fix-loop bound: three iterations then escalate.** Stages 4-6
  loop into stage 7 at most three times; on the third non-shipping panel result
  the agent presents to the approval queue WITH the remaining objections,
  escalating to the operator. Shapes R4 and acceptance criterion 6.
- **D5 (RESOLVED) -- channels, role-split.** `mesh` carries durable STATE and
  COORDINATION (stage posts, intent/interest, claims, peer awareness; survives
  agent death) -- the agent->driver and agent<->peer channel. `SendMessage`
  carries live, context-preserving operator STEERING of a running agent (the
  `redo` path, SPEC-ORCH-001 TB-011). Operator verb: "message the job"/`redo` ->
  SendMessage; durable state -> mesh.

## 7. Remaining open decisions

- None blocking. Implementation (stage 3) proceeds per PLAN-FACTORY-001 milestones
  M1-M6.

## 8. Closure

CLOSED -- see `CLOSE.md` (CLOSE-FACTORY-001). The factory ran its own eight-stage
lifecycle to completion: concept -> plan -> implement (M1-M6) -> test (live mesh
dedup, AC4 PASS) -> persona analysis (5 reviewers, SHIP WITH CAVEATS) -> evaluate ->
fix (F1 enum, F2 glossary pointer) -> present and close. Design accepted and wired;
AC4 live-verified, AC7 release-verified, AC1-3/5-6/8 wired. One honest gap remains:
a single continuous end-to-end factory run (AC1) is not yet demonstrated -- the
natural next validation when desired.
