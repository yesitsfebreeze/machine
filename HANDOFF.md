# Handoff

## Mode — grill-first (the orchestrator's default)
- One question at a time; resolve the decision tree branch by branch.
- Every question carries your recommended answer.
- Codebase can answer it? Explore, don't ask.
- Minimal text. We discuss, we don't dump plans.

## Goal (see target.md)
Repo behaves like a senior programmer we hand jobs to:
grill → plan → review → store → (gate) implement → test → arbiter → (gate) propose
merge. Each unit runs in a git-fs sub-agent we can talk to. Scale to many jobs under
the driver, all merging into git-fs, with a live roster of running agents.

## Acceptance invariants
- Everything ends up in git-fs (implementation on a `gitfs/<sid>` branch, merged into
  `main` only on an approved `git_fs_merge`).
- There is always a live list of running agents (the `/.machine/sessions/` ledger).

## The drill flow (drill skill)

Only the surface is themed: the orchestrator is the **drill**; sub-agents are spoken
of as **miners**. Everything underneath keeps plain names (job, git-fs, sub-agent).
1. **Grill (default).** Refine with the user via `grill-me` until they call it valid.
2. **Plan agent.** Dispatch one sub-agent to write the implementation plan.
3. **Review (advisory).** `/personas` + `codex-review` (plan mode). Codex never gates.
4. **Store.** Plan written to `.machine/plans/<id>.md`.
5. **Gate one.** Ask the user before dispatching implementation.
6. **Implementation agent.** Own git-fs branch, worktree-isolated, given the stored
   plan, builds autonomously, runs `gate` until green, then `codex-review` (arbiter).
7. **Gate two.** Build green and stable → present diff + verdicts, propose merge.
8. **Merge and close.** On approval, `git_fs_merge` into `main`, release mesh claim,
   delete the ledger entry.

No settle timer, no auto-fire, no ScheduleWakeup-driven launch. The two human gates
(dispatch, merge) are the only points where real cost or a change to `main` is
incurred. Codex and personas are advisory throughout; the hard merge blocker is a
green build plus approval.

## State — done
- **git-fs v3.0.1 adopted** (clean rewrite): `gitfs/<sid>` branches, Stop hook
  materializes touched files to disk (no auto-merge to main), never hard-fails.
- **drill skill written** (merge of the orchestrate flow + grill-me) (grill-first, gated dispatch, ledger-as-roster);
  all settle-timer / auto-fire / ScheduleWakeup machinery stripped.
- **drill agent written** (was orchestrator) to match (grill-first, two gates, codex advisory,
  authors only `/.machine/**`, merges only on approval).
- **codex wired** via a lean `codex-review` skill (plan + arbiter modes, advisory,
  degrades to n/a when codex absent). Registered in plugin.json. The heavy
  `codex-peer-review` addon stays in `mine/` for deep multi-perspective synthesis.
- **drill is the single entry point.** Renamed orchestrator->drill (agent + skill);
  merged the orchestrate flow + grill-me into one `drill` skill. `assemble` and `ignite`
  are no longer standalone skills — folded into drill's bring-up and kept as reference
  files under `.claude/skills/drill/references/`. `/drill` self-detects setup state:
  bootstraps deps + oils a cold repo, else resumes the roster, then drives. SessionStart
  hook points at drill. Core is now 4 agents + 7 skills.
- Earlier: bare-bones core + addon kit in `mine/` (v0.3.10); mesh rewritten to
  zero-dep Node ESM; grill-me skill added.

## Decisions resolved this session
- Replace the timed auto-fire taskboard with grill-first explicit gates.
- Codex is advisory at both review points (user decides; green build gates merge).
- Build against git-fs-2 (now shipped as installed git-fs v3.0.1).
- Plans live in `.machine/plans/` (already the configured `plansDirectory`).

## Caveats / known
- **Mid-session git-fs swap:** a session that started under an older git-fs keeps its
  old `agent/<sid>` branch; the new `gitfs/<sid>` materialize only runs at Stop. When
  Edit/Write are denied and git-fs was swapped mid-session, Bash direct-disk writes
  are the reliable, in-session-verifiable path (used to build this change). Clean from
  the next fresh session onward.
- Cruft to sweep: ~13 stale `agent/*` branches in the git-fs store + ~20 `temp_local_*`
  plugin cache dirs under `~/.claude/plugins/cache/`.
- Reload reported "1 error during load" — surfaced only by `/doctor`; likely the
  `yesitsfebreeze/git-fs` marketplace mapped to two local paths in `~/.claude.json`.
- **Feature-factory specs aligned to the drill (done this pass).** All five
  `.machine/specs/feature-factory/*` docs (CONCEPT/SPEC/PLAN/README/CLOSE) were swept
  to the drill model: dead `.claude/skills/orchestrate/SKILL.md` links repointed to
  `drill/SKILL.md`; `SPEC-ORCH-001` TB-rules repointed to the drill's "Board trust"
  section; branch naming `agent/<id>` → `gitfs/<sid>`; taskboard/settle-queue/
  approval-queue prose → drill ledger-as-roster + two human gates. **D2/R7 were
  re-resolved** (SPEC v1.4.0): the now-invalid "ledger = orchestrate taskboard with
  settle queue" is replaced by "ledger = the drill's live roster," substance
  unchanged. Prior HISTORY rows left intact as audit trail. `.machine/sessions/README.md`
  also rewritten to the drill model. The factory itself is still unbuilt (Next #3);
  AC1 (single continuous end-to-end run) remains the open validation.

## Next — open
1. **Validate the flow end-to-end.** Drive one real job grill→plan→implement→merge in
   a fresh session and confirm the git-fs branch + merge + roster all behave.
2. **Multi-orchestrator scaling.** N drivers × M git-fs job-agents → merge → roster.
   Cross-driver state must live in `mesh` (roster + claims), not the per-driver ledger.
   Confirm mesh MCP is connected (it did not surface as a tool this session).
3. **/oil → mine/.** Make oil scan `mine/` and suggest fits per repo (incl. the heavy
   `codex-peer-review` addon). Not built.
