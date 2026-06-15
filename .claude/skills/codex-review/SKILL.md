---
name: codex-review
description: Second-AI review via the Codex CLI at the drill's two checkpoints — plan review and the end-of-implementation arbiter. Advisory only: it returns notes and concerns for the driver and user to weigh, it never gates, blocks, merges, or edits. Skips cleanly when codex is not installed. Trigger via "/codex-review", "codex review", "second opinion from codex", or invoked by the drill skill.
---

# codex-review

A thin wrapper over the Codex CLI for a second-AI perspective at the two
drill review points. It is advisory: the verdict is attached to what the
user sees, it never gates a decision, blocks a merge, or changes files. The hard
blocker for a merge stays the green build plus the user's approval.

For deep multi-perspective synthesis (architecture critique, alternative-approach
generation), the heavier `codex-peer-review` addon in `mine/` is the tool; this
skill is the lean checkpoint integration the drill calls.

## Availability

Run only when codex is on PATH. If `command -v codex` is empty, record the verdict
as `n/a`, say codex was unavailable, and proceed with Claude-only review. Never
block on a missing codex.

## Two modes

### plan — review a stored or proposed plan

Used in the drill flow after the plan agent returns, alongside the persona
panel. Pipe the plan and a focused question to `codex exec` (non-interactive,
reads the prompt from stdin):

    cat .machine/plans/<id>.md | codex exec "Review this implementation plan. Flag unstated assumptions, missing steps, risky sequencing, and simpler alternatives. Be specific and concise."

Map the result to the entry's `review.codex`: `notes` (suggestions worth folding
in) or `concerns` (a reviewer would not proceed as written). Feed material notes
back to the plan agent via SendMessage, or fold them in the next grill turn.

### arbiter — review the implementation diff

Used at the end of implementation, after the build is green, before the merge is
proposed. Run a non-interactive code review against the agent's branch diff:

    codex review

or, against an explicit diff:

    git_fs diff <main> <gitfs/sid>  →  cat <diff> | codex exec "Code review this diff for correctness, security, and reuse/simplification. Concise, line-level where possible."

Map the result to `review.codex` and attach it to the merge proposal next to the
gate result and persona synthesis. It rides along; it does not decide.

## What this skill never does

- It never blocks the pipeline: a codex `concerns` verdict informs the user, it does
  not halt a dispatch or a merge.
- It never edits files or merges branches.
- It never runs when codex is absent — it degrades to `n/a` silently.

## Synthesis

Present codex's view transparently next to Claude's: where they agree (higher
confidence), where they diverge (a trade-off to surface), and what one caught that
the other missed. Do not force consensus; the user is the arbiter.
