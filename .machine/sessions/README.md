# /.machine/sessions/ — orchestrator taskboard

One Markdown entry-file per task, named `<id>.md` (`a1`, `a2`, ...). Together these
files ARE the orchestrator's taskboard — there is no separate board file. Written
and read by the `orchestrate` skill, and read by the `ignite` SessionStart hook to
resume open work. This directory is the single source of truth for the
orchestrator's timed attention footer.

Each entry is a *task with a timer*, not only a finished-subagent record: it
carries a settle countdown (`fire_at`) and auto-fires when the countdown elapses
unless the user intervenes, then — once fired and validated — waits for approval.

This directory converges toward empty: a task's file is **deleted** when it is
approved or dropped — never marked done. A clean board is an empty directory (this
README aside).

An entry also doubles as the **feature ledger** when a factory agent owns a full
lifecycle: it carries `stage` (lifecycle position), `branch` (the git-fs
`agent/<id>` it builds on), and `claim_id` (the mesh claim). The factory agent
never writes here — by board trust only the driver does; the driver projects the
agent's `mesh`-reported stage onto the entry. See the skill for the field
definitions.

See `.claude/skills/orchestrate/SKILL.md` for the entry-file schema, the status
lifecycle, the settle countdown, the ScheduleWakeup scheduler, and the
add / edit / freeze / approve / drop / show / redo commands. Do not duplicate that
schema here — update the skill, not this note.

These are ephemeral working files. They are intentionally not meaningful history;
once resolved they are gone.
