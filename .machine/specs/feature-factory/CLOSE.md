---
id: CLOSE-FACTORY-001
spec: SPEC-FACTORY-001
title: "Feature factory — closure (stage 8: present and close)"
status: accepted
created: 2026-06-15
author: machine-default (loop @target, iteration 9)
---

# Feature Factory — Closure

Stage 8 of the factory's own lifecycle, applied to itself. The factory ran the
full eight stages: concept (CONCEPT-FACTORY-001) -> plan (PLAN-FACTORY-001) ->
implement (M1-M6) -> test (live mesh dedup) -> persona analysis (5-reviewer panel)
-> evaluate -> fix (F1 enum, F2 glossary pointer) -> present and close (this doc).

> Amended (post-rewrite): references below to the `orchestrate` taskboard and
> SPEC-ORCH-001 TB-rules now read as the **drill's roster** and the drill's
> board-trust rule (drill/SKILL.md); see SPEC-FACTORY-001 D2 (v1.4.0). The
> shipped substance is unchanged -- only the vocabulary and source pointers moved.

## What shipped

- **G1 -- lifecycle runner = the `default` agent.** Two dispatch roles in
  `.claude/agents/default.md`: stage dispatch (one unit, report back) and
  factory-job dispatch (own one feature's full eight-stage lifecycle on its own
  git-fs branch). Owning one lifecycle is not orchestrating (D6).
- **G2 -- feature ledger = the drill's roster.** `.claude/skills/drill/SKILL.md`
  entry-file schema carries `stage`/`branch`/`claim_id`; the drill is sole ledger
  writer and reconciles the factory agent's `mesh` posts (amended R9), preserving
  the drill's board-trust rule.
- **G3 -- dedup handshake.** `roster` + `claims` + `claim` + intent `post` before
  stage 1; on a held claim, post deferred-interest and stand down (D3).
- **Decisions:** D1/D2/D6 (roles + ledger), D3 (stand down + register interest),
  D4 (three-iteration fix-loop bound then escalate), D5 (mesh = durable
  state/coordination, SendMessage = live operator steering).
- **Vocabulary:** five glossary terms in `glossary.csv`.

## Acceptance criteria -- status with evidence

| AC | Statement (abbrev) | Status | Evidence |
|----|--------------------|--------|----------|
| 1 | Agent walks all 8 stages unaided | WIRED | default.md lifecycle; handshake+plan stages demonstrated in the live test, full end-to-end single run not yet exercised |
| 2 | Stage observable in mesh + reflected in ledger | WIRED + mesh side shown | live test broadcast `stage:plan`; driver->ledger reconciliation is instruction-level |
| 3 | Entry resolves to one branch + one claim_id | WIRED | schema fields; not runtime-exercised |
| 4 | Same feature -> exactly one builds | **LIVE-VERIFIED PASS** | alice won claim 01KV537JT3..., bob stood down (D3); driver-verified claims=[] after release |
| 5 | Gate passes before merge to main | WIRED | default.md stage 5/8 + the drill's gate two; this build's own stage-4 gate passed |
| 6 | Fix loop bounded, non-shipping never reaches close | WIRED | D4 three-iteration bound in default.md stage 7 |
| 7 | Close releases claim + closes entry; no orphan claim | PARTIAL-VERIFIED | release verified live (claims=[]); ledger-close is instruction-level |
| 8 | No parallel tracking structure | VERIFIED | ledger IS the drill's roster; no new directory created |

## Honest remaining gap

The one thing not yet demonstrated as a single continuous run is **AC1** -- one
dispatched factory agent driving all eight stages end to end on its own git-fs
branch (the live test exercised the handshake and the plan stage, plus dedup). The
natural next validation is a single-feature end-to-end factory run.

## Deferred (pre-existing, out of factory scope -- separate cleanup pass)

context7 `get-library-docs` renamed to `query-docs` (default.md stale); `superpowers:*`
skills are an undeclared external dependency; the old board addon was retired and
replaced by the local taskboard addon (`.machine/taskboard.json`, resolved per-cwd at
runtime); `glossary.csv` rows 10/11/26-30 have unquoted commas (non-blocking);
`settle_delay` double-defined. All logged to kern.

## Verdict

Design accepted, wired, and partially live-verified. Panel verdict: SHIP WITH
CAVEATS; the two realized drifts are fixed. Closed.
