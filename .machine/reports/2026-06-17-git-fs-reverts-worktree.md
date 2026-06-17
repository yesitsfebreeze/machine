# git-fs silently reverts the working tree mid-edit, destroying uncommitted work

- **date:** 2026-06-17
- **severity:** blocker
- **area:** git-fs (plugin / virtual filesystem daemon)
- **status:** open
- **issue:** https://github.com/yesitsfebreeze/machine/issues/1

## What happened
While building the `hub machine` subcommand in `hub/src/machine.rs`, git-fs
repeatedly rolled the on-disk file back to a STALE earlier version between a Read
and the following Edit. It wiped applied work (double-tap detach: `detach_key`,
`DOUBLE_TAP_MS`) from the working tree and even resurrected a file deleted with a
plain `rm` (`scripts/machine.mjs`). Edit/Read tooling kept failing with "File has
been modified since read" because git-fs mutated the file underneath the edit.

## Expected
The working tree should hold what was last written/edited until the author changes
or commits it. git-fs should not overwrite uncommitted local edits to tracked files.

## Evidence
```
# disk reverted vs the committed/pushed HEAD:
$ git show HEAD:hub/src/machine.rs | grep -c 'DOUBLE_TAP_MS\|fn detach_key'   -> 3
$ grep -c 'DOUBLE_TAP_MS\|fn detach_key' hub/src/machine.rs                   -> 0
$ echo "disk: $(wc -l < hub/src/machine.rs)  HEAD: $(git show HEAD:... | wc -l)"
  disk: 493  HEAD: 553
$ git status --short hub/src/machine.rs   ->  M hub/src/machine.rs

# plain rm of a tracked file came back on disk AND was re-committed:
$ test -e scripts/machine.mjs   -> EXISTS on disk   (after `rm` earlier)
```
Edit tool also returned: "File has been modified since read, either by the user or
by a linter. Read it again before attempting to write it." on consecutive edits.

## Context
- repo: /home/feb/dev/machine, branch `main`, agent role: orchestrator/default
- OS: Linux WSL2; git-fs installed as a companion plugin (`.git-fs/` present)
- Reproduced several times across one session; only a `git commit` immediately
  after writing survived. Recovery that worked: `git checkout HEAD -- <path>`.

## Suspected cause / fix (optional)
git-fs appears to periodically sync/checkout tracked files from its virtual store
back onto disk, clobbering uncommitted edits (and re-materializing rm'd files).
Likely needs: (a) skip files with unstaged local modifications during its sync,
or (b) honor plain-FS deletes, or (c) a guard/lock so external edits aren't
overwritten. Until fixed, the safe workflow is: edit from `git show HEAD:<path>`,
then `git add && git commit` immediately; use `git rm` (not `rm`); the commit is
the only durable artifact.
