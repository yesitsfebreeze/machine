---
name: report
description: >
  File a machine self-diagnostic report as a tagged GitHub issue. When a tool,
  daemon, hook, skill, or agent hangs, errors, returns garbage, or behaves wrong,
  open a `machine-report`-labeled issue on the machine plugin repo so every plugin
  user's reports funnel to the maintainer. Requires `gh` logged in. Write-side only
  — it logs, it does not fix. Trigger: "/report", "report a problem", "this tool is
  broken", "file a machine report", "log this hang/error/hiccup".
metadata:
  version: "2.0.0"
  category: "workflow"
  status: "active"
  updated: "2026-06-17"
  tags: "report, bug, hang, broken, diagnostic, self-fix, tooling, hiccup, github, issue"
---

# /report — file a machine defect as a tagged GitHub issue

The machine should learn from its own friction. When a tool hangs, a daemon won't
come up, a hook misfires, a skill does the wrong thing, or any part of the machine
hiccups — **don't just work around it silently**. File a report.

Reports are **GitHub issues**, not local files. A local file under `/.machine/` is
only fixable by someone who forked the machine and runs your exact setup — useless
to the maintainer and to every other install. An issue on the **machine plugin
repo** funnels every user's reports to one place, where they can actually be triaged
and fixed for everyone.

This skill is **write-only**. It opens the issue. It does **not** triage or fix —
that is `/resolve`, which only runs in the machine repo itself.

## Requirements

`gh` must be installed **and authenticated**. Check `gh auth status`. If it fails,
do **not** fall back to a local file — tell the operator to run `gh auth login`,
report that the issue could not be filed, and continue the original task. No login,
no report.

## When to file

File a report when the machine itself misbehaves:

- a tool/MCP call hangs, times out, or returns an error that isn't your input's fault
- a daemon (kern, hub) won't launch, drops, or returns nonsense
- a hook fires wrong, blocks, or never fires
- a skill or agent does something clearly off-spec
- a documented capability doesn't work as written
- repeated friction that a machine change would remove

Do **not** use it for: ordinary code bugs in the *target* project (those are normal
work), or feature ideas (those are brainstorm/drill). Issues are for the machine's
own defects.

## Where reports go

Always the **machine plugin repo**, never the user's current repo — pass `--repo`
explicitly so the issue lands upstream regardless of the user's cwd remote:

```
yesitsfebreeze/machine
```

Labels (the "tags"): `machine-report` always, plus `severity:<severity>`
(`blocker` | `major` | `minor` | `annoyance`). Ensure both labels exist first —
create them on demand so the flow works on a fork too.

## Issue format

**Title:** one factual line naming the problem.

**Body:** keep it short — what broke, not prose.

```markdown
- **severity:** blocker | major | minor | annoyance
- **area:** <tool / daemon / hook / skill / agent name>

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

1. **Check auth.** `gh auth status`. If it fails, stop (see Requirements).
2. **Dedupe.** Search open reports for the same problem:
   `gh issue list --repo yesitsfebreeze/machine --label machine-report --state open --search "<keywords>"`.
   If a matching issue exists, **comment** on it (`gh issue comment <url> --body ...`)
   instead of opening a near-duplicate — single source of truth applies to issues too.
3. **Pick** an area and a severity. Fill **Evidence** from the actual tool output you
   saw — quote it, don't paraphrase from memory.
4. **Ensure labels, then open the issue** (substitute `<title>`, `<severity>`, and
   write the body to a temp file or pass it inline):

   ```bash
   REPO=yesitsfebreeze/machine
   gh label create machine-report --color B60205 \
     --description "Filed by /report — machine self-diagnostic" --repo "$REPO" 2>/dev/null || true
   gh label create "severity:<severity>" --color CCCCCC --repo "$REPO" 2>/dev/null || true
   gh issue create --repo "$REPO" \
     --title "<title>" \
     --body "<body in the format above>" \
     --label machine-report --label "severity:<severity>"
   ```

5. **Confirm** the issue URL (`gh issue create` prints it) back to the operator in one
   line. Then continue the original task — filing a report does not abort your work,
   you note it and move on.

## The consume side (machine repo only)

`/report` only ever writes. To act on the backlog, run `/resolve` **in the machine
repo** — it lists the open `machine-report` issues, fixes the top one at its root
cause, and closes that issue. `/resolve` is repo-local to the machine and is not
shipped to other installs.
