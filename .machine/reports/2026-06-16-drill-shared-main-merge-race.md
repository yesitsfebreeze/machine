# Drill merge lands via update-ref on the shared main checkout, racing concurrent sessions

- **date:** 2026-06-16
- **severity:** major
- **area:** skill/drill (merge/close step) + multi-session coordination
- **status:** open

## What happened
Two drill sessions ran against the same repo at once: a1 (replace trello with
taskboard) and a2 (marketplace/terms refactor). Both landed work on `main` from the
single shared working checkout. The drill skill prescribes `git_fs_merge`, but the
miner committed with plain git, so the driver merged at the ref level. The driver
(this session) advanced `main` with `git update-ref` against the live shared checkout,
and computed the merged tree against a `main` revision that a2 then moved. Net result:
- a2's `git add -A` commit reverted a1's taskboard files from main's tree even though
  the a1 merge commit was in the DAG ancestry (lost-update);
- a1's corrective commit, computed against an older `main`, then dropped a2's
  `marketplace.json` (+16) and README additions (lost-update in the other direction);
- the shared working checkout was left with a stale index that read as a "staged
  revert" of taskboard (trello present on disk, taskboard missing), confusing to commit.
It took three recompute passes and a final `git reset --hard main` (after confirming no
genuine uncommitted work) to converge with both sessions' work intact.

## Expected
Landing a feature on `main` should never drop a concurrent session's committed work,
never corrupt the human's shared working tree, and never depend on `main` holding
still mid-merge. Two drivers should serialize on `main`.

## Evidence
- `merge-base HEAD agent/a1` == agent/a1 head (git thinks "already merged") yet
  `main:mine/skills/taskboard/SKILL.md` was absent — a2's add -A reverted post-merge.
- First CAS commit: `main now: cc1af780d6` had already moved from the `200461e` the
  tree was computed against → `git diff cc1af78 main` showed `marketplace.json | 16 ---`
  (a2 work dropped by a1's stale tree).
- Reflog: `200461e@{0}  87f7d61@{1}(merge)  00b37ae@{2}` — interleaved commits from two
  sessions seconds apart (13:03–13:12), all on `refs/heads/main`.
- Fix required `merge-tree --write-tree --merge-base=<agent/a1^> <latest-main> agent/a1`
  + `commit-tree` + `update-ref <new> <old>` (CAS), recomputing against latest main.

## Context
Repo: machine (this plugin repo), branch main, OS Linux, driver = drill main loop.
git 2.43, no git-fs branch in play (miner used plain git, not git_fs_write). Single
shared working checkout used by both sessions. kern lesson stored as
`lesson:drill-shared-main-race`.

## Suspected cause / fix
1. Drill close/merge must NOT touch the human's shared main checkout (no update-ref /
   checkout / reset there) — it leaves a stale index. Land via git-fs ref merge or a
   dedicated throwaway worktree, never the shared tree.
2. Ref-level merge must be a bounded recompute+CAS loop: on each attempt read latest
   `main`, compute the 3-way tree with `--merge-base=<feature-parent>` against that
   exact `main`, `commit-tree`, then `update-ref <new> <old-oid>`; on CAS failure
   recompute against the new `main` (never commit a tree computed against a stale main).
3. Serialize drivers: take a `mesh` claim on resource `main` (or `branch:main`) before
   any ref update so only one driver lands at a time; release after.
4. Enforce the drill invariant that miners edit via git-fs (per-edit commits on their
   branch) so merges go through `git_fs_merge`, not ad-hoc plain-git ref moves.
5. Consider a guard: refuse to `update-ref main` if the repo has another live drill
   session registered on mesh without holding the `main` claim.
