---
name: report
description: >
  File a machine self-diagnostic report. When a tool, daemon, hook, skill, or
  agent hangs, errors, returns garbage, or behaves wrong, write a structured
  report into /.machine/reports/ so the machine can later be pointed at the folder
  and told to fix itself. Write-side only — it logs, it does not fix. Trigger:
  "/report", "report a problem", "this tool is broken", "file a machine report",
  "log this hang/error/hiccup".
metadata:
  version: "1.0.0"
  category: "workflow"
  status: "active"
  updated: "2026-06-16"
  tags: "report, bug, hang, broken, diagnostic, self-fix, tooling, hiccup"
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
5. Confirm the path back to the user/operator in one line. Then continue the
   original task (filing a report does not abort your work — you note it and move on).

## The consume side (manual, not this skill)

To act on the backlog, the operator opens a fresh session and says something like
"read `/.machine/reports/`, triage the open ones, and fix the machine." That pass
reads each `status: open` report, fixes the root cause in the machine, flips the
report to `status: fixed` (or removes it), and commits. `/report` only ever writes.
