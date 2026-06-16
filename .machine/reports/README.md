# Machine self-diagnostic reports

Defects in the **machine itself** — tool hangs, daemon failures, hook misfires,
off-spec skills/agents — land here as one `<date>-<slug>.md` file each, filed by
the `/report` skill. Not for target-code bugs (normal work) or feature ideas (drill).

To act on the backlog: open a session and say "read `.machine/reports/`, triage the
open ones, and fix the machine." That pass fixes root causes, flips each report to
`status: fixed`, and commits. `/report` only writes here; fixing is a separate pass.
