---
name: report
description: >
  File a machine self-diagnostic report. When a tool, daemon, hook, skill, or
  agent hangs, errors, returns garbage, or behaves wrong, write a structured
  report into /.machine/reports/ AND mirror it to a tagged GitHub issue on the
  machine plugin repo so every plugin user's reports funnel to the maintainer.
  Write-side only — it logs, it does not fix. Trigger: "/report", "report a
  problem", "this tool is broken", "file a machine report", "log this
  hang/error/hiccup".
metadata:
  version: "1.1.0"
  category: "workflow"
  status: "active"
  updated: "2026-06-17"
  tags: "report, bug, hang, broken, diagnostic, self-fix, tooling, hiccup, github, issue"
---

# /report — file a machine self-diagnostic report

The machine should learn from its own friction. When a tool hangs, a daemon won't
come up, a hook misfires, a skill does the wrong thing, or any part of the machine
hiccups — **don't just work around it silently**. Drop a report. Later the operator
points the machine at `/.machine/reports/` and says "read these and fix yourself".

This skill is **write-only**. It captures the problem. It does **not** triage or
fix — that is a separate, deliberate pass the operator drives.

## When to file

File a report when the machine itself misbehaves:

- a tool/MCP call hangs, times out, or returns an error that isn't your input's fault
- a daemon (kern, mesh) won't launch, drops, or returns nonsense
- a hook fires wrong, blocks, or never fires
- a skill or agent does something clearly off-spec
- a documented capability doesn't work as written
- repeated friction that a machine change would remove

Do **not** use it for: ordinary code bugs in the *target* project (those are normal
work), or feature ideas (those are brainstorm/drill). This folder is for the
machine's own defects.

## Where reports go

One file per report under `/.machine/reports/`:

```
/.machine/reports/<YYYY-MM-DD>-<short-slug>.md
```

Slug: 3-6 kebab words naming the problem (`mesh-launch-hang`, `gate-lint-false-positive`).
If the same problem recurs, append a short note to the existing file rather than
spawning a near-duplicate — single source of truth applies to reports too.

## Report format

Write the file with these sections. Keep it factual and short — what broke, not prose.

```markdown
# <one-line title of the problem>

- **date:** <YYYY-MM-DD>
- **severity:** blocker | major | minor | annoyance
- **area:** <tool / daemon / hook / skill / agent name>
- **status:** open
- **issue:** <github issue URL — filled after the issue is opened, or `none` if gh unavailable>

## What happened
<2-4 lines: what you did, what the machine did instead.>

## Expected
<1-2 lines: what should have happened.>

## Evidence
<exact command/tool call + the error/output. A few lines, not a dump.>

## Context
<repo, OS, branch, agent role, anything that helps reproduce.>

## Suspected cause / fix (optional)
<only if you actually have a lead — otherwise omit.>
```

## How to run

1. Pick the area and a severity.
2. Ensure `/.machine/reports/` exists (create it if missing).
3. Compute the filename: today's date + slug. Check for an existing file on the
   same problem first; append instead of duplicating.
4. Write the report using the format above. Fill **Evidence** from the actual
   tool output you saw — quote it, don't paraphrase from memory.
5. **Mirror it to a GitHub issue** (see below). Write the returned URL into the
   report's **issue:** field. If the same problem already has an issue, comment on
   that issue instead of opening a duplicate.
6. Confirm the path **and** the issue URL back to the user/operator in one line.
   Then continue the original task (filing a report does not abort your work — you
   note it and move on).

## The GitHub issue mirror

Every report is also opened as a tagged issue on the **machine plugin repo**
(`yesitsfebreeze/machine`), never the user's current repo — that way reports from
every install funnel to the maintainer, and anyone can browse or filter them by the
`machine-report` label.

Target repo: `yesitsfebreeze/machine` (the machine plugin's upstream). Always pass
`--repo` explicitly so the issue lands there regardless of the user's cwd remote.

Labels (the "tags"): `machine-report` always, plus `severity:<severity>`. Ensure
both labels exist first — create them on demand so the flow works on a fork too. The
issue body is the report file verbatim, so the local `.md` and the issue never drift.

Run, after the report file is written (substitute `<path>`, `<title>`, `<severity>`):

```bash
REPO=yesitsfebreeze/machine
gh label create machine-report --color B60205 \
  --description "Filed by /report — machine self-diagnostic" --repo "$REPO" 2>/dev/null || true
gh label create "severity:<severity>" --color CCCCCC --repo "$REPO" 2>/dev/null || true
gh issue create --repo "$REPO" \
  --title "<title>" \
  --body-file "<path>" \
  --label machine-report --label "severity:<severity>"
```

`gh issue create` prints the issue URL — capture it for step 5.

**Graceful degradation.** If `gh` is missing or unauthenticated (`gh auth status`
fails), skip the issue, set the report's **issue:** field to `none`, tell the
operator the issue could not be filed, and keep going. The local file always wins;
the issue is the public mirror.

## The consume side (manual, not this skill)

To act on the backlog, the operator opens a fresh session and says something like
"read `/.machine/reports/`, triage the open ones, and fix the machine." That pass
reads each `status: open` report, fixes the root cause in the machine, flips the
report to `status: fixed` (or removes it), closes the linked GitHub issue, and
commits. `/report` only ever writes.
