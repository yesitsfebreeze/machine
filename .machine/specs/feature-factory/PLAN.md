---
id: PLAN-FACTORY-001
title: "Feature factory — implementation plan (stage 2)"
spec: SPEC-FACTORY-001
version: 1.0.0
status: draft
created: 2026-06-15
updated: 2026-06-15
author: machine-default (loop @target, iteration 4)
priority: high
---

# Feature Factory — Implementation Plan

Stage-2 plan for `SPEC-FACTORY-001`. The factory is a documentation-and-config
change to the machine payload (agent prompt + orchestrate skill + glossary), not
application code. "Build" here means configuration integrity: references resolve,
`settings.json` parses, hooks pass `node --check`, English-only, no emoji.

This plan grounds G1/G2/G3 in the real taskboard schema (`SPEC-ORCH-001`,
TB-001..TB-014) and resolves the conflict that schema creates with the factory's
ledger writes.

## 0. Decisions resolved by this plan

- **D3 (RESOLVED) -- lost-the-claim behaviour: stand down + register interest.**
  When the G3 handshake finds the feature's `mesh` claim already held by a live
  peer, the agent does NOT begin stage 1. It `post`s a deferred-interest note to
  `mesh` (so the holder and the driver see a second agent wanted the feature) and
  exits cleanly, leaving no ledger entry in an active stage. No automatic takeover
  in v1 (a queue-and-inherit mechanism is deferred -- it risks two agents both
  believing they will run the feature). The driver or operator re-dispatches if the
  holder dies or releases.
- **D4 (RESOLVED) -- fix-loop bound: three iterations, then escalate.** Stages 4-6
  loop into stage 7 and back until the persona panel returns a ship verdict OR
  three fix iterations have run. On the third non-shipping result the agent stops
  looping and presents the work to the approval queue (stage 8) WITH the panel's
  remaining objections attached, escalating the decision to the operator rather
  than looping unbounded.
- **D5 (RESOLVED) -- communication channels, role-split.** Two channels, each with
  a distinct job, consistent with the existing architecture:
  - **`mesh`** (`post`/`inbox`/`read`, `claim`/`release`, `roster`) -- the durable,
    survives-agent-death channel for STATE and COORDINATION: stage-transition
    broadcasts, intent/interest posts, claims, and peer awareness. This is the
    agent->driver and agent<->peer channel.
  - **`SendMessage`** -- the live, context-preserving channel for the operator (or
    driver) to STEER a running factory agent: the `redo <id>: <note>` path already
    defined in `SPEC-ORCH-001` TB-011. Use it to send direction to an in-flight
    agent without restarting it from zero.
  Operator-facing verb: "message the job" / `redo` routes through `SendMessage` to
  the live agent; durable cross-session state always flows through `mesh`.

## 1. The G2/TB conflict and its resolution (load-bearing)

`SPEC-FACTORY-001` R9 had the factory agent update its own ledger entry's `stage`
field. But `SPEC-ORCH-001` TB-013/TB-014 make the taskboard **driver-owned**: only
the driver creates, schedules, or writes entry-files under `/.machine/sessions/`,
and any entry a dispatched agent writes is quarantined as `untrusted` and never
auto-fired. A factory agent is a dispatched agent, so it must not write the ledger.

**Resolution (amends R9):** the factory agent NEVER writes the ledger. It `post`s
each stage transition to `mesh` (R3, unchanged). The **driver** is the sole ledger
writer: on its turn -- woken by the mesh post or a background-agent notification --
it reconciles the agent's reported stage into that entry's `stage`/`branch`/
`claim_id` fields. This keeps TB-013/TB-014 intact, keeps the board-trust safety
model whole, and makes `mesh` the load-bearing agent->driver state channel (D5).

Consequence: ledger and mesh can be briefly out of sync between the agent's post
and the driver's next turn; this is acceptable and bounded by the driver's wake
cadence. The mesh post is the source of truth for stage; the ledger is the
driver's durable projection of it.

## 2. Milestones (priority-ordered, no time estimates)

1. **M1 -- Ledger schema delta (G2 / R7, R8, R10).** Extend the taskboard
   entry-file frontmatter (`SPEC-ORCH-001` TB-001 minimum fields) with three
   driver-written fields: `stage` (enum: concept|plan|implement|test|personas|
   evaluate|fix|present; orthogonal to `status`), `branch` (`git-fs` `agent/<id>`),
   `claim_id` (mesh claim handle). Document in `.claude/skills/orchestrate/SKILL.md`
   schema and `/.machine/sessions/README.md`. No new directory; no parallel
   structure.
2. **M2 -- Driver-owned reconciliation (G2 / amended R9).** Specify in
   `orchestrate/SKILL.md` that the driver updates a factory entry's `stage`/
   `branch`/`claim_id` from the agent's `mesh` posts; the factory agent never
   writes `/.machine/sessions/`. Reconciles with TB-013/TB-014.
3. **M3 -- Dedup handshake + stand-down (G3 / R11-R14, D3).** Formalize the
   pre-stage-1 ritual already sketched in `default.md` ("Running jobs in
   parallel"): `roster` + `claims` + `claim` + intent `post`; on a held claim,
   stand down and `post` deferred-interest (D3). Add the stand-down detail to
   `default.md`.
4. **M4 -- Fix-loop bound (D4 / R4).** Update `default.md` stage-7 wording: loop
   stages 4-6 -> 7 at most three times, then present with remaining objections.
5. **M5 -- Communication channels (D5).** Document the mesh/SendMessage role split
   in `default.md` (factory-job dispatch section) and `orchestrate/SKILL.md`.
6. **M6 -- Glossary + memory (machine law).** Add terms to `/.machine/glossary.md`
   (or `.csv`): factory agent, factory-job dispatch, feature ledger, dedup
   handshake, stage (lifecycle). Ingest the resolved decisions into `kern`.

## 3. Integration points

- `.claude/agents/default.md` -- stage-7 bound (M4), stand-down detail (M3),
  channel role split (M5). G1/D6 two-role split already landed (iteration 3).
- `.claude/skills/orchestrate/SKILL.md` -- ledger schema delta (M1), driver
  reconciliation rule (M2), channel note (M5).
- `/.machine/sessions/README.md` -- describe `stage`/`branch`/`claim_id` (M1).
- `/.machine/glossary.md` -- new terms (M6).

## 4. Risks

- **Ledger/mesh skew.** The driver-reconciliation model means `stage` lags the
  agent's true position between driver turns. Bounded and acceptable; mesh is the
  source of truth. Do not add a second writer to "fix" the lag -- that reopens the
  TB-013 hole.
- **Stand-down races (D3).** Two agents handshake near-simultaneously; both could
  see a free claim before either `claim` lands. Mitigated by `mesh` claim being an
  atomic CAS (`SPEC-COMM-001`): exactly one `claim` wins; the loser sees the held
  claim on its own read-after-write and stands down.
- **Unbounded fix loop without D4.** Guarded by the three-iteration ceiling (M4).
- **Scope creep into orchestration.** A factory agent must own ONE lifecycle, not a
  fleet (D6) -- no taskboard writes, no nested factory dispatch. TB-013/TB-014 are
  the backstop.

## 5. Definition of Done

- All SPEC-FACTORY-001 requirements (R1-R14, R9 as amended) trace to a milestone.
- The ledger carries `stage`/`branch`/`claim_id`, written only by the driver
  (TB-013 intact); the factory agent writes no file under `/.machine/sessions/`
  (TB-014 intact).
- The handshake stands down + registers interest on a held claim (D3); the fix
  loop is bounded at three iterations (D4); the two channels are documented with
  their distinct roles (D5).
- Glossary updated; decisions ingested into kern; configuration-integrity gate
  passes (references resolve, settings parse, hooks `node --check`).

## 6. Next concrete step

All milestones M1-M6 DONE (stage 3 implement complete). Next: stage 4 verify the
config-integrity gate (references resolve, settings parse, hooks node --check,
English-only/no-emoji), then stage 5 run the persona panel against the factory
changes, evaluate (stage 6), and fix (stage 7) before present/close (stage 8).
