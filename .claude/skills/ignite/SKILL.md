---
name: ignite
description: |
  Machine bring-up at session start. Idempotent: brings up caveman comms, verifies the repo is oiled (/.machine present), nudges /oil-me when it is not, and enters orchestration mode when it is. Invoked automatically by the ignite SessionStart hook; also runs on "ignite", "/ignite", "bring up the machine", "start machine mode".
when_to_use: Fired by the ignite SessionStart hook every session, or on explicit "ignite" / "/ignite" / "start machine mode". Run once per session. Skip the orchestration step when /.machine is absent.
---

# Ignite — machine bring-up

Single entry point for a session. Run the steps in order, quietly. Do not narrate
each step; act, then give one short status line. Honor any state the hook passed
in `additionalContext` (oiled or not, open subagents list).

## 1. Comms

Caveman comm mode is ON by default. Invoke the `caveman` skill (default level:
full) and follow it for all responses. Off only on explicit user request:
"stop caveman" / "normal mode".

## 2. Setup check (idempotent)

Check whether `/.machine` exists in the project root.

- **Absent (not oiled):** this repo has the machine plugin but no project layer.
  Tell the user once, briefly, that the repo is not oiled and offer to run the
  `oil-me` skill. Do **not** enter orchestration. Stop here.
- **Present (oiled):** continue to step 3. Do not re-run oil-me; setup is done.

The statusbar, output style, and env are supplied by the plugin settings file —
ignite does not touch them. If the statusbar is missing, that is a plugin-settings
problem, not an ignite step.

## 3. Orchestration (only when oiled)

Enter orchestration mode: invoke the `orchestrate` skill. If the hook listed open
subagents from a prior session (status pending-approval / running /
changes-requested), rebuild the "needs your attention" footer from
`/.machine/sessions/` and resume tracking them. If there were none, enter
orchestration ready but idle — do not invent work.

## 4. Status line

Close with one compact line, e.g.:
`machine: oiled · caveman full · orchestration on · 2 open subagents`
or when unoiled:
`machine: not oiled — run /oil-me to specialize this repo`
