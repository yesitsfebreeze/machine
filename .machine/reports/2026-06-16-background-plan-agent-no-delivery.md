# Report: background Plan agent idles "available" without delivering its result

- **Date:** 2026-06-16
- **Severity:** medium (blocks the drill plan stage when a plan agent is backgrounded)
- **Component:** Agent tool (run_in_background) + Plan subagent type + drill flow

## What happened
The drill dispatched a `Plan` subagent (`board-planner`) with `run_in_background: true`
to write an implementation plan. The agent emitted three `idle_notification`s
(`interrupted`, then `available` x2) but never delivered any plan content to the
driver. Two explicit `SendMessage` requests asking it to route the full plan to
`main` produced another idle "available" with no content. `TaskList` then showed no
tasks (the agent had terminated), and no Agent-tool result was ever surfaced to the
driver.

## Impact
The driver received zero usable output from a completed background plan agent. The
plan stage stalled; the driver had to abandon the subagent and author the plan
inline instead.

## Suspected cause
A backgrounded `Plan` subagent returns its plan as its final Agent-tool message, but
when it goes idle/terminates in background mode the final text is not routed to the
driver, and the `Plan` agent type does not proactively `SendMessage("main", ...)`
with its result. Net: a backgrounded read-only architect agent can complete without
its artifact ever reaching the driver.

## Workaround used
Stopped relying on the subagent; the driver wrote the plan directly from prior
research. For plan-stage work, prefer either a foreground Agent call (so the result
returns synchronously) or a spawn prompt that explicitly instructs the agent to
`SendMessage("main", <full plan>)` before going idle.

## Suggested fix
- Drill skill: when dispatching a plan agent in background, the spawn prompt MUST
  instruct it to deliver the final artifact via `SendMessage("main", ...)`, not just
  as plain final text.
- Consider running plan agents in the foreground by default (the driver waits on one
  short architect pass) rather than backgrounding them.
