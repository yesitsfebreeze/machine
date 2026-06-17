> Reference for the `drill` skill's bring-up — the per-session entry playbook drill follows on start. Not a registered skill; drill consults it.

# Ignite — machine bring-up

Single entry point for a session. Run the steps in order, quietly. Do not narrate
each step; act, then give one short status line. Honor any state the hook passed in
`additionalContext` (oiled or not, open roster).

## 1. Comms

Caveman comm mode is ON by default. Invoke the `caveman` skill (default level: full)
and follow it for all responses. Off only on explicit user request: "stop caveman" /
"normal mode".

## 2. Setup check (idempotent)

Check whether `/.machine` exists in the project root.

- **Absent (not oiled):** this repo has the machine plugin but is not set up yet. Tell
  the user once, briefly, and offer to run the `assemble` skill — it bootstraps
  dependencies and configuration, then oils the project layer. (If only the project
  layer needs writing, `oil` alone suffices.) Do **not** enter drill mode. Stop here.
- **Present (oiled):** continue to step 3. Do not re-run oil; setup is done.

The statusbar, output style, and env are supplied by the plugin settings file — ignite
does not touch them.

## 3. Drill mode (only when oiled)

Enter drill mode: invoke the `drill` skill. The drill creates its own single worktree
off `main` (`git worktree add /.machine/worktrees/drill-<sid> -b drill/<sid> main`) and
operates from it — it never works in or `checkout`s the human's main checkout, and every
feature it drives is built inside that one worktree as a `gitfs/<id>` branch (no per-miner
worktree). The live roster lives in the hub, not on disk: rebuild the footer from
`mcp__hub__roster` + `mcp__hub__claims` and project it onto the board. That skill is the
single source of truth for the hub-backed roster.

Then scan `/.machine/worktrees/` for stale orchestrator worktrees left by dead sessions
(a `drill-*` whose session is no longer live) and offer to remove them
(`git worktree remove` + prune the branch) — only on explicit user approval. Nothing
auto-fires; enter ready and await user commands. If there were none, enter drill mode
ready but idle — do not invent work.

## 4. Status line

Close with one compact line, e.g.:
`machine: oiled · caveman full · drill on · 2 open jobs`
or when unoiled:
`machine: not set up — run /assemble to bootstrap this repo`
