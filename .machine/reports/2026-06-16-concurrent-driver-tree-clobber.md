# Concurrent main-loop drivers clobber each other's working tree

- **date:** 2026-06-16
- **severity:** major
- **area:** drill / git-fs / session isolation
- **status:** open

## What happened
Two main-loop driver sessions committed to `main` on the same shared working tree at
the same time: this `/improve`-loop driver and an `[a1]` taskboard session. The `[a1]`
session ran a merge that reset the working tree to HEAD, silently wiping this driver's
**uncommitted** tracked edits to `.claude/hooks/personas.mjs` and
`.claude/hooks/subagent-report.mjs` (a dedup refactor in flight). Untracked files (the
new `.claude/hooks/stop-input.mjs`) and all committed work survived; only uncommitted
tracked edits were lost. The harness surfaced it only as after-the-fact "file modified
externally" reminders — no warning before the overwrite.

## Expected
A driver's in-progress edits should not be silently destroyed by another session's git
operations. Concurrent sessions should be isolated (own git-fs branch + worktree), or a
direct-to-`main` edit should be guarded so a competing merge cannot reset the tree under
an active editor.

## Evidence
Clobbering commit on `main`:
```
e0afc0d fix(merge): fold taskboard onto a2 marketplace work — undo stale-tree clobber [a1]
```
Post-merge `git status` showed my edited hooks reverted to HEAD (line 3 back to
`import { readFileSync } from "fs";`), while untracked `stop-input.mjs` remained:
```
?? .claude/hooks/stop-input.mjs
?? .machine/plans/
```
Re-applied + committed the lost edits as `a82c147`.

## Context
- Repo: `machine` (this repo), branch `main`, Linux/WSL.
- Agent role: main-loop driver running `/loop 10m /improve`.
- A second driver `[a1]` (taskboard) and an `[a2]` (marketplace) session were committing
  to `main` concurrently.

## Suspected cause / fix
Multiple drivers share one repo working tree with no lock or worktree isolation. git-fs
is designed to isolate via `gitfs/<sid>` branches, but these sessions committed directly
to `main`'s working tree. This race has bitten at least twice: the glossary records the
"sole driver" law for `improve.json` (incident e193515d), and `[a1]`'s own commit message
says "undo stale-tree clobber". Fix direction: each concurrent session operates on its own
`gitfs/<sid>` branch + worktree and never edits `main`'s working tree directly; OR a
working-tree lock guards direct-to-`main` edits so a competing merge/checkout cannot reset
the tree while another driver has uncommitted changes.
