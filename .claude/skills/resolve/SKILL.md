---
name: resolve
description: >
  Consume the machine self-diagnostic backlog. Reads the open reports in
  /.machine/reports/, ranks them by severity/impact, fixes the single highest-priority
  one at its root cause, verifies green, then deletes that report file. The act side
  of /report (which only writes). Repo-local skill — not part of the portable machine.
  Trigger: "/resolve", "resolve a report", "triage the reports", "fix the top report",
  "work the reports backlog".
metadata:
  version: "1.0.0"
  category: "workflow"
  status: "active"
  updated: "2026-06-16"
  tags: "resolve, report, triage, self-fix, backlog, machine-defect, consume"
---

# /resolve — fix the top machine report, then delete it

`/report` is write-only: it files defects into `/.machine/reports/`. `/resolve` is the
**act side**. One invocation works exactly **one** report — the highest priority open
one — fixes the root cause in the machine, proves the tree is green, and removes that
report file. Run it again to take the next one.

This is a **repo-local** skill: it lives in `.claude/skills/resolve/` but is
deliberately **not** registered in `.claude-plugin/plugin.json`, so it does not travel
when the machine plugin is installed elsewhere.

## Scope guard

- Works only on files under `/.machine/reports/` (ignore `README.md`).
- Fixes defects in the **machine itself** — tools, daemons, hooks, skills, agents,
  config. Never treats a report as a target-code feature request.
- One report per run. Do not batch-fix; each defect deserves its own verification and
  commit.
- The fix and the report deletion land on the shared `main` tree: hold the `mesh`
  `branch:main` claim while you edit and commit, release after, and stand down if a
  live peer holds it (@.claude/shared/main-lock.md).
- Root cause, never a patch (machine law). If a report only has symptoms, investigate
  until the actual cause is found before editing.

## How to run

0. **Collect.** Always start by running the collector — it scans, parses, and ranks the
   open backlog for you:

   ```
   python3 @../tools/collect_reports.py
   ```

   It prints JSON (`{count, reports:[...]}`) with reports already ordered
   highest-priority first. If `count` is 0, say the backlog is empty and stop.
1. **Gather.** Take the collector's `reports[]` as the backlog (it already excludes
   `README.md` and non-`open` reports). Do not re-list the directory by hand.
2. **Pick.** Take `reports[0]` (rank 1) as the default top. The collector ranks by
   `severity` (blocker > major > minor > annoyance) then oldest date. Before committing,
   read that report's body and sanity-check breadth of impact and recurrence — if a
   lower-ranked report is clearly more urgent, override and say why in one line.
3. **Diagnose.** Read the report's Evidence and Suspected cause. Reproduce or trace the
   defect in the actual machine files before changing anything. Query kern for prior
   decisions on the affected area first.
4. **Fix.** Implement the root-cause fix in the machine (the relevant skill/agent/hook/
   config/daemon). Keep exactly one clean implementation; remove any obsolete code in
   the same change.
5. **Verify.** Run `/gate` (or the narrowest check that exercises the fix) and quote the
   passing output. No "done" without evidence. If the defect was in a hook/daemon/skill,
   exercise that path directly.
6. **Close the issue.** If the report has an **issue:** field with a URL, close it:
   `gh issue close <url> --repo yesitsfebreeze/machine -c "Fixed: <one-line root cause>."`
   Skip silently if the field is `none`/absent or `gh` is unavailable.
7. **Delete.** Remove the resolved report file from `/.machine/reports/`. Deleting is the
   close signal for this skill (unlike the generic consume pass, which flips
   `status: fixed`).
8. **Report back.** One-line summary: which report, the root cause, the fix, gate result,
   and how many reports remain in the backlog.

## After the fix

- If the fix changed a documented capability, update the single source of truth (the
  skill/agent doc or `/.machine`), not a copy.
- Record the decision in kern (`mcp__kern__ingest`) when the daemon is up — what defect,
  what root cause, what changed — so the next session does not re-decide it.
- If working the report surfaces a *new* distinct defect, file it with `/report` rather
  than expanding this run's scope.
