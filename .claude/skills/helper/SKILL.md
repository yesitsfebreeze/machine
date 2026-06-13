---
name: helper
description: Capture a recurring repo task as a tiny project-local helper skill, and register its trigger tags so it fires reliably next time. Use when workflow friction recurs, when the helper-suggest hook nudges, or when the user asks to "make a helper" / "remember how we do X here". Helpers live in /.machine/skills/ (project-local, not the portable machine).
---

# Helper — capture a recurring task so it goes right next time

A **helper skill** is a tiny project-local doc (`/.machine/skills/<name>.md`) that
teaches the one correct way to do a task that recurs in THIS repo — the command,
the gotcha, the order of steps. It is NOT a portable `.claude` skill; it stays in
the project layer and is re-indexed by `/oil-me`.

Reliable triggering is the whole point: helpers are wired to a **tag cloud**
(`/.machine/skills/registry.json`). The `helper-trigger.mjs` UserPromptSubmit hook
matches each prompt against the tags and force-injects a reminder to READ the
helper before acting. That is deterministic — it does not depend on model
judgment the way ordinary skill-description matching does.

## When to invoke

- The `helper-suggest` Stop hook nudged about workflow friction.
- A task failed, took many retries, or you improvised a method — and it will
  recur in this repo.
- The user says "make a helper", "remember how we do X here", "capture this".

## The loop (ask first — never write unprompted)

1. **Name the recurring task** in one sentence. If it was a one-off (typo,
   external outage, a thing that won't repeat), STOP — say so and do not create a
   helper. Noise helpers rot the tag cloud.
2. **Ask the user** whether to capture it, and confirm the task name + the
   trigger tags. Do not skip this — capture is collaborative by design.
3. On approval, **author + register**: follow `reference/authoring.md`.
4. **Confirm** the registry entry and remind the user it triggers on next match.

## Hard rules

- Helpers live ONLY in `/.machine/skills/` — never in `.claude/skills/` (that is the
  portable machine and would leak project specifics to other repos).
- One helper = one recurring task. Keep it tiny: the correct method and the
  gotcha, not a tutorial.
- Every helper MUST be registered in `registry.json` with tags, or it never
  triggers. An unregistered helper is dead weight.
- Tags are the words that naturally appear in a prompt about the task. Choose
  them so a real future prompt will hit them; avoid tags so generic they fire on
  everything (e.g. "code", "file", "run").
- Glossary discipline: if the helper introduces or fixes a term, update
  `/.machine/glossary.csv` too.

See `reference/authoring.md` for the exact file shape and registry schema.
