# /.machine/sessions/ — the drill's ledger (live roster)

One Markdown entry-file per job, named `<id>.md` (`a1`, `a2`, ...). Together these
files ARE the roster of running agents — there is no separate board file. Written
and read by the `drill` skill, and read at session start to resume open work.

There is no timer. An entry carries no `fire_at`, no settle countdown, and nothing
auto-fires. Each entry records where a job sits — grilling, planning, plan-review,
implementing, arbiter, or awaiting one of the two human gates (dispatch, merge).
Work starts only when the user chooses to start it; `main` changes only on an
approved `git_fs_merge`.

This directory converges toward empty: a job's file is **deleted** when it is
merged or dropped — never marked done. A clean roster is an empty directory (this
README aside).

An entry also doubles as the **feature ledger** when a factory agent owns a full
lifecycle: it carries `stage` (lifecycle position), `branch` (the git-fs
`gitfs/<sid>` it builds on), and `claim_id` (the mesh claim). The factory agent
never writes here — by board trust only the drill does; the drill projects the
agent's `mesh`-reported stage onto the entry.

See `.claude/skills/drill/SKILL.md` for the entry-file schema, the status
lifecycle, and the drill commands. Do not duplicate that schema here — update the
skill, not this note.

These are ephemeral working files. They are intentionally not meaningful history;
once resolved they are gone.
