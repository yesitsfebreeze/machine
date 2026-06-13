# /.proj/sessions/ — orchestrator state board

One Markdown file per background subagent, named `<id>.md` (`a1`, `a2`, ...).
Written and read by the `orchestrate` skill. This directory is the single source
of truth for the orchestrator's attention footer.

This directory converges toward empty: a subagent's file is **deleted** when its
work is approved or dropped — never marked done. A clean board is an empty
directory (this README aside).

See `.claude/skills/orchestrate/SKILL.md` for the file schema, status lifecycle,
and the approve / redo / show / drop commands. Do not duplicate that schema here —
update the skill, not this note.

These are ephemeral working files. They are intentionally not meaningful history;
once resolved they are gone.
