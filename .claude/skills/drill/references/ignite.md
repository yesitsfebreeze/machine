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

Enter drill mode: invoke the `drill` skill. If the hook listed an open roster from a
prior session (any non-terminal status: grilling / planning / plan-review / plan-ready
/ implementing / arbiter / merge-proposed), rebuild the roster footer from
`/.machine/sessions/`. Reconcile every pre-existing entry-file through the `drill`
skill's board-trust model — that skill is the single source of truth for how a
pre-existing entry is handled (untrusted until the user adopts it). Nothing auto-fires;
enter ready and await user commands. If there were none, enter drill mode ready but
idle — do not invent work.

## 4. Status line

Close with one compact line, e.g.:
`machine: oiled · caveman full · drill on · 2 open jobs`
or when unoiled:
`machine: not set up — run /assemble to bootstrap this repo`
