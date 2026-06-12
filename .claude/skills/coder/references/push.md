# Phase 7 — Incremental pushes

Keep remote in lockstep with local. Don't hoard.

Push = upstream counterpart of bounded commits. Same discipline: small, frequent, traceable.

## Cadence

Push after every meaningful checkpoint:
- Each green-refactor cycle completing a coherent slice
- Before stepping away (lunch, EOD, context switch)
- Before any operation that could lose work (rebase, branch surgery, reboot)

Rule: **local >5 commits ahead of origin → push.**

## Why

- **Backup.** Local-only = one disk failure from gone.
- **CI early signal.** Each push exercises pipeline; catch breakage on the causing commit.
- **Visibility.** Reviewers see direction without waiting.
- **Bisect on origin works.** Mega-pushes collapse history.
- **Rollback granularity.** Reverting one pushed commit = cheap.

## Don't

- Hoard until "feature done" — defeats backup, CI, visibility
- Push and force-push casually — destructive on shared branches
- Push broken/half-typed without marking — others may pull. WIP → `wip/<topic>` or `[WIP]` PR title
- Mix branches in one push session — one branch, one push, one outcome

## Force-push rules

Never force-push to `main`/`master` or shared branch. Confirm with user before any force-push.

Acceptable on own feature branch:
- Rewriting history before review (squash typos, reword subjects)
- Rebase onto updated base

Use `--force-with-lease`, never blind `--force`. Lease aborts if someone pushed in between.

## After push

- Watch CI. Pipeline break → fix forward in new commit, don't amend-and-force.
- Review finding → fresh commit on same branch.

## Stop

Local + remote agree. CI green on latest pushed. No work older than last push exists only on disk.

## Anti-patterns

- Push-once-at-end — defeats every purpose
- Silent force-push to shared — overwrites others
- Pushing without watching CI — failures compound
- Pushing broken `main` — use topic branch for WIP
