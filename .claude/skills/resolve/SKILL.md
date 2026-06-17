---
name: resolve
description: >
  Consume the machine self-diagnostic backlog. Lists the open `machine-report`
  GitHub issues on the machine repo, ranks them by severity then age, fixes the
  single highest-priority one at its root cause, verifies green, then closes that
  issue. The act side of /report (which only writes). Machine-repo-only skill — not
  shipped to other installs. Trigger: "/resolve", "resolve a report", "triage the
  reports", "fix the top report", "work the reports backlog".
metadata:
  version: "2.0.0"
  category: "workflow"
  status: "active"
  updated: "2026-06-17"
  tags: "resolve, report, triage, self-fix, backlog, machine-defect, consume, github, issue"
---

# /resolve — fix the top machine report issue, then close it

`/report` is write-only: it opens defect issues on the machine repo. `/resolve` is
the **act side**. One invocation works exactly **one** issue — the highest-priority
open `machine-report` — fixes the root cause in the machine, proves the tree is
green, and closes that issue. Run it again to take the next one.

This is a **machine-repo-only** skill. It lives in `.claude/skills/resolve/` but is
deliberately **not** listed in `.claude-plugin/plugin.json` `skills[]`, so it is not
shipped when the machine plugin is installed elsewhere — it only resolves the
machine's own repo, where the fixes actually land.

## Requirements

`gh` installed and authenticated (`gh auth status`), with push access to
`yesitsfebreeze/machine`. If auth fails, stop and tell the operator to `gh auth login`.

## Scope guard

- Works only on open `machine-report` issues on `yesitsfebreeze/machine`.
- Fixes defects in the **machine itself** — tools, daemons, hooks, skills, agents,
  config. Never treats a report as a target-code feature request.
- One issue per run. Do not batch-fix; each defect deserves its own verification and
  commit.
- The fix lands on the shared `main` tree: hold the `hub` `branch:main` claim while
  you edit and commit, release after, and stand down if a live peer holds it
  (@.claude/shared/main-lock.md).
- Root cause, never a patch (machine law). If an issue only has symptoms, investigate
  until the actual cause is found before editing.

## How to run

1. **List.** Pull the open backlog, newest-first with labels:

   ```bash
   gh issue list --repo yesitsfebreeze/machine --label machine-report --state open \
     --json number,title,labels,createdAt,url --limit 100
   ```

   If the list is empty, say the backlog is empty and stop.
2. **Rank & pick.** Order by severity (`severity:blocker` > `major` > `minor` >
   `annoyance`, read from each issue's labels) then oldest `createdAt`. Take the top
   as the default. Read its full body (`gh issue view <number> --repo yesitsfebreeze/machine`)
   and sanity-check breadth of impact and recurrence — if a lower-ranked issue is
   clearly more urgent, override and say why in one line.
3. **Diagnose.** Read the issue's Evidence and Suspected cause. Reproduce or trace the
   defect in the actual machine files before changing anything. Query kern for prior
   decisions on the affected area first.
4. **Fix.** Implement the root-cause fix in the machine (the relevant skill/agent/hook/
   config/daemon). Keep exactly one clean implementation; remove any obsolete code in
   the same change.
5. **Verify.** Run `/gate` (or the narrowest check that exercises the fix) and quote the
   passing output. No "done" without evidence. If the defect was in a hook/daemon/skill,
   exercise that path directly.
6. **Close.** Close the issue with a one-line root-cause note:
   `gh issue close <number> --repo yesitsfebreeze/machine -c "Fixed in <commit/branch>: <root cause>."`
7. **Report back.** One-line summary: which issue, the root cause, the fix, gate result,
   and how many issues remain in the backlog.

## After the fix

- If the fix changed a documented capability, update the single source of truth (the
  skill/agent doc or `/.machine`), not a copy.
- Record the decision in kern (`mcp__kern__ingest`) when the daemon is up — what defect,
  what root cause, what changed — so the next session does not re-decide it.
- If working the issue surfaces a *new* distinct defect, file it with `/report` rather
  than expanding this run's scope.
