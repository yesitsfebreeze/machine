---
name: foundation-quality
description: >
  Code quality discipline for the machine: the five-dimension quality gate,
  proactive review habits, and the verification checklist that must pass before
  work is called done. Use for code review and quality-gate checks.
license: Apache-2.0
compatibility: Designed for Claude Code
allowed-tools: Read, Grep, Glob, mcp__context7__resolve-library-id, mcp__context7__get-library-docs
user-invocable: false
metadata:
  version: "3.0.0"
  category: "foundation"
  status: "active"
  updated: "2026-06-12"
  tags: "foundation, quality, review, verification, quality-gate"
progressive_disclosure:
  enabled: true
  level1_tokens: 100
  level2_tokens: 5000
triggers:
  keywords: ["quality", "code review", "quality gate", "linting", "coverage", "verification", "code smell", "technical debt"]
  agents:
    - "manager-quality"
    - "expert-testing"
    - "expert-security"
    - "expert-refactoring"
    - "evaluator-active"
---

# Foundation Quality

How the machine keeps quality high: one gate, reviewed proactively, verified by
evidence. The runnable check is `/gate` (it detects the project's
fmt/lint/test/build, or reads the exact commands from `/.machine/project.md`); the
five dimensions below are the language-agnostic standard `/gate` enforces, shared
with `foundation-core`.

## The quality gate — five dimensions

Every non-trivial change must satisfy all five before completion:

- **Tested** — behavior is covered and the suite passes; new functionality ships
  with a test. Uncovered new code is the riskiest code.
- **Readable** — names and structure make intent obvious; the linter is clean,
  with any remaining warning suppressed per-line and justified, never globally.
- **Unified** — formatting and imports match the project; exactly one current
  implementation, no leftover duplicate.
- **Secured** — user input and auth paths checked against the OWASP top ten
  (`ref-owasp-checklist`); no unbounded input or concurrency hazard. Escalate to
  `expert-security` when unsure.
- **Trackable** — a clear, conventional commit message; the change is traceable
  to its reason.

For a deeper review of an existing diff use `/code-review`; for cleanup-only
passes use `/simplify`. Coverage strategy lives in `ref-testing-pyramid`.

## Review proactively

- **Shift left** — the earlier a defect is found, the cheaper it is. Quality
  checks belong in the development loop, not bolted on before release.
- **Chesterton's fence** — before removing a check or guard, understand why it
  was added. Removing it blind repeats the failure it was built to prevent.
- Automation catches syntax and known patterns; a human-style review pass catches
  design flaws, naming confusion, and missing abstractions. Do both.

## Common rationalizations

| Rationalization | Reality |
|---|---|
| "The linter warnings are false positives" | Suppress them per-line with a reason. Ignoring them trains the team to ignore real issues. |
| "Security scanning can wait until before release" | Vulnerabilities compound; late discovery means expensive rework. Check continuously. |
| "Coverage is high enough, the rest is edge cases" | Edge cases are where production bugs live. The uncovered code is the riskiest code. |
| "Code review is subjective, automation is sufficient" | Automation catches patterns; review catches design flaws and missing abstractions. |
| "The gate is too bureaucratic for a hotfix" | A hotfix without the gate breeds the next hotfix. The gate is the minimum, not the maximum. |

## Red flags

- Linter or type-checker warnings suppressed globally instead of per-line.
- OWASP checklist not consulted when handling user input or authentication.
- No coverage for a commit that adds new functionality.
- A gate dimension skipped as "not applicable" without justification.
- A quality finding identified but left with no action.

## Verification before "done"

- [ ] Linter clean, or remaining warnings carry inline suppression with reasons.
- [ ] OWASP checklist reviewed for security-relevant changes.
- [ ] Coverage generated and threshold met — quote the tool output.
- [ ] All five gate dimensions assessed.
- [ ] Findings triaged with a resolution for each.
- [ ] No global rule disabling in the linter config.

Claim completion only after running the checks and quoting the output
(`superpowers:verification-before-completion`).
