---
name: codex-review
description: Second-AI review via the Codex CLI at the drill's single review point — the consolidated review of the finished diff, alongside the persona panel, before a landing is proposed. Advisory only: it returns notes and concerns for the driver and user to weigh, it never gates, blocks, merges, or edits. Skips cleanly when codex is not installed. Trigger via "/codex-review", "codex review", "second opinion from codex", or invoked by the drill skill.
---

# codex-review

A thin wrapper over the Codex CLI for a second-AI perspective at the drill's one
review point — the consolidated review of the materialized diff, run with the persona
panel after the build is green and before a landing is proposed. It is advisory: the
verdict is attached to what the user sees, it never gates a decision, blocks a landing,
or changes files. The hard blocker for landing stays the green build plus the user's
approval.

For deep multi-perspective synthesis (architecture critique, alternative-approach
generation), the heavier `codex-peer-review` addon in `mine/` is the tool; this
skill is the lean checkpoint integration the drill calls.

## Availability

Run only when codex is on PATH. If `command -v codex` is empty, record the verdict
as `n/a`, say codex was unavailable, and proceed with Claude-only review. Never
block on a missing codex.

## Review the diff

Run at the drill's single review point: after the build is green, before the landing
is proposed, scaled to the size of the change. Run a non-interactive code review
against the miner's branch diff:

    codex review

or, against an explicit diff:

    git_fs diff <main> <gitfs/id>  →  cat <diff> | codex exec "Code review this diff for correctness, security, and reuse/simplification. Concise, line-level where possible."

Map the result to the job's `review.codex` (`notes` — suggestions worth folding in, or
`concerns` — a reviewer would not proceed as written) and attach it to the landing
proposal next to the gate result and persona synthesis. It rides along; it does not
decide. Material notes go back to the miner via SendMessage if the user wants them
addressed before landing.

## What this skill never does

- It never blocks the pipeline: a codex `concerns` verdict informs the user, it does
  not halt a landing.
- It never edits files or merges branches.
- It never runs when codex is absent — it degrades to `n/a` silently.

## Synthesis

Present codex's view transparently next to Claude's: where they agree (higher
confidence), where they diverge (a trade-off to surface), and what one caught that
the other missed. Do not force consensus; the user is the arbiter.
