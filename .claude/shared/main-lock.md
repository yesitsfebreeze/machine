# Main-tree lock — serialize every write to the shared `main`

The repo-root working tree on `main` is shared by every session in the repo. Two
sessions that edit, commit, merge, checkout, or reset it concurrently clobber each
other: one session's merge/checkout resets the tree to HEAD and silently destroys
another session's uncommitted tracked edits, or a ref update computed against a
`main` that a peer just moved drops committed work (lost-update). This has bitten
more than once (incidents e193515d, e0afc0d). The fix is mutual exclusion on the
shared tree, realized through the `mesh` bus (full verbs: @.claude/shared/mesh.md).

## The rule

Before you edit, commit, merge, checkout, reset, or `update-ref` the shared
repo-root `main` working tree, you MUST hold the `mesh` claim on resource
`branch:main`. Release it the moment you are done landing. This binds every session
type — drill drivers at close/merge, the `/improve` loop, `/resolve`, and any direct
editor of `main`.

You do NOT need the lock when your writes go to your own isolated worktree on a
`gitfs/<sid>` / `drill/<sid>` branch under `/.machine/worktrees/` — that is the whole
point of worktree isolation. The lock guards only the one shared tree.

## The protocol

1. **Acquire.** `mcp__mesh__register` (refresh liveness), then `mcp__mesh__claims`
   to inspect, then `mcp__mesh__claim` the resource `branch:main`.
2. **If a live peer already holds it,** do not edit or land on `main`. `mcp__mesh__post`
   a deferred-interest note to the holder and to `*`, then either wait for your queued
   claim to be granted or stand down — never edit the shared tree under another holder.
3. **Land.** With the claim held, make your edits and commit, or run the ref merge.
   Recompute against the current `main` tip immediately before the ref update; never
   commit a tree computed against a stale `main`.
4. **Release.** `mcp__mesh__release` the `branch:main` claim as soon as the landing is
   committed, so a waiting peer proceeds. Hold it for the landing, not for your whole
   session.

## Why a claim, not a file lock

`mesh` already arbitrates resource locks with liveness and a wait queue, so a dead
holder's claim does not wedge the repo and a waiting session is granted in turn. A
bare lockfile has neither property. This generalizes the older `sole driver` law
(which covered only `improve.json`) to the entire shared `main` working tree.
