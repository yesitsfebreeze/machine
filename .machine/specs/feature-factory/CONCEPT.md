---
id: CONCEPT-FACTORY-001
title: "Feature factory — the repo as a dispatchable senior programmer"
version: 0.1.0
status: draft
created: 2026-06-15
updated: 2026-06-15
author: machine-default (loop @target, iteration 1)
priority: high
---

# Feature Factory

> Note (post-rewrite): this concept predates the orchestrate->drill rename. The
> "orchestrate taskboard" it refers to is now the **drill's live roster**; see
> SPEC-FACTORY-001 D2 (v1.4.0) for the re-resolved ledger model. Body updated to
> drill vocabulary; the design intent is unchanged.

## 1. The target

From `target.md`: the repo should behave like a real senior programmer we hand
jobs to — a generalist who knows everything, not a specialist scoped to one
domain. Given a job in any domain, that programmer runs a full lifecycle end to
end:

1. writes a concept
2. writes an implementation plan
3. implements it
4. tests it
5. lets the persona panel analyze it
6. evaluates what needs to change
7. implements the adjustments and fixes
8. presents the result and closes it

The whole lifecycle runs inside a subagent we can communicate with. We dispatch
several such jobs at once, isolate each on its own git-fs branch, track open
features, and let the agents talk so two of them never build the same thing twice.

## 2. What already exists

The primitives are almost all in place. The factory is mostly wiring, not new
infrastructure.

| Target need | Existing machinery |
|---|---|
| Memory of past decisions (the "why") | `kern` daemon + SessionStart digest |
| Live coordination, awareness, chat | `mesh` daemon (SPEC-COMM-001) |
| Atomic dedup — "don't build it twice" | `mesh` `claim` / `claims` / `roster` |
| Per-job isolation, edit-as-commit, merge-on-stop | `git-fs` companion plugin |
| Drill-first dispatch + two human gates | `drill` skill + ledger (roster) |
| Stage 1 — concept | `superpowers:brainstorming`, `manager-spec` |
| Stage 2 — plan | `manager-strategy`, `superpowers:writing-plans` |
| Stage 3 — implement | `manager-tdd` / `manager-ddd` + `expert-*` |
| Stage 4 — test | `expert-testing`, `gate` skill |
| Stage 5 — persona analysis | `personas` skill + `/.machine/personas/` |
| Stage 6 — evaluate | `evaluator-active`, `plan-auditor` |
| Stage 7 — fixes | `expert-*` agents |
| Stage 8 — present and close | `manager-git`, gate + the drill's merge gate |

## 3. The gap

No single dispatchable unit chains stages 1-8. Today an operator drives each
agent by hand through the drill skill. The target wants one job = one
self-driving lifecycle. Three concrete pieces are missing:

- **G1 — the lifecycle runner.** A skill/agent that, given a one-line job, walks
  all eight stages itself, dispatching the right specialist per stage and gating
  each handoff. The drill skill is the drill-first driver but does not encode
  this fixed 8-stage chain.
- **G2 — the feature ledger.** RESOLVED (D2): the drill's roster under
  `/.machine/sessions/` IS the ledger, extended with `stage`, `branch`, and
  `claim_id` fields so each entry maps a mesh claim to a feature, its lifecycle
  stage, and its git-fs branch. No parallel structure.
- **G3 — the dedup handshake.** A startup ritual every factory agent runs before
  stage 1: `roster` + `claims` to see what peers hold, `claim` the feature, and
  `post` an intent broadcast. If the claim is already held, the agent stands down
  or queues instead of duplicating work.

## 4. Proposed shape (D1 RESOLVED — the default agent is the runner)

The lifecycle runner is **the `default` agent itself** — no separate `factory`
skill or agent type. The user's decision: the default eager-generalist (whole
toolbelt, bias-to-verify, a generalist who knows everything and pulls in
specialists only for depth) IS the senior programmer. The 8-stage job loop is
baked into the default agent definition (`.claude/agents/default.md`,
section "The job lifecycle"). G1 is therefore behavior in the agent prompt, not
a new artifact.

Given a concrete job, the default agent:

1. runs the G3 dedup handshake against `mesh` (`roster` + `claims` + `claim` +
   intent `post`); stands down or queues if the feature is already held;
2. on a clean claim, works on its git-fs `gitfs/<sid>` branch and (G2) records the
   feature, stage, and branch;
3. drives stages 1-8 itself, delegating a single stage to a specialist only for
   depth, and posting progress to `mesh` as it crosses each stage;
4. validates with `gate` + `personas` (stages 4-6), looping fixes until the panel
   ships it;
5. presents and closes (stage 8) at the drill's merge gate (gate two), merging via
   git-fs;
6. releases the `mesh` claim.

Multiple jobs run as parallel default-agent sessions, each on its own branch, each
visible to the others through the roster and the ledger.

## 5. Open decisions (next iterations)

- D1: RESOLVED — the `default` agent is the runner; the 8-stage loop is baked into
  its definition. No separate skill or agent type.
- D2: RE-RESOLVED (v1.4.0) — reuse the drill's roster under `/.machine/sessions/` as
  the single feature ledger. Added three fields to its entry-file schema: `stage`
  (lifecycle position, orthogonal to the drill's `status`), `branch` (git-fs
  `gitfs/<sid>`), and `claim_id` (mesh claim handle). No new directory.
- D3: How a queued (lost-the-claim) agent behaves — stand down, watch, or assist
  the holder.
- D4: Whether stages 5-7 loop until the persona panel returns a ship verdict, and
  the max-iteration bound.

## 6. Next concrete step

Write `SPEC-FACTORY-001` formalizing G1-G3 in EARS form, then draft the `factory`
skill against it. Resolve D1 first — it shapes everything downstream.
