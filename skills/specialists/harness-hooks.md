# harness-hooks

**When**: configuring PreToolUse / PostToolUse / UserPromptSubmit / SessionStart hooks.
**Why care**: hooks are how the harness enforces invariants the model can't be trusted to remember.

## Decision tree
- "From now on do X" automated behavior → SessionStart or UserPromptSubmit hook. Reason: model can't self-enforce.
- Block dangerous tool calls → PreToolUse. Reason: only chance to deny before execution.
- Post-process tool output → PostToolUse. Reason: redact / summarize / log.
- Inject session-wide context → SessionStart. Reason: stable prefix, cache-friendly.

## Tradeoffs
- Hook-enforced rules: deterministic. Pay: bypass requires editing settings.
- Memory/preferences alone: drift over long sessions.
- Heavy hooks: latency on every event; keep them fast.

## Anti-patterns (why)
- Network call in PreToolUse: blocks every tool call on remote latency.
- Writing prompts to hooks instead of skills: hooks are control, skills are guidance.
- Silent hook failures: model never learns the rule didn't apply.
