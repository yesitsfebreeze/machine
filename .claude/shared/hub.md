# Hub — the agent coordination bus

Every agent in a session shares one hub bus (the `hub` MCP server). It is how
parallel agents avoid building the same thing twice, and how they talk to each
other and to the driver. Your `agent_id` is your spawn / git-fs branch id.

## The eight verbs

- `mcp__hub__register` — announce your id, branch, and prompt pointer; refreshes
  your liveness heartbeat. Call once on start, and again on long runs to stay live.
- `mcp__hub__roster` — list known agents and who is currently live.
- `mcp__hub__claims` — inspect current resource locks and their queues.
- `mcp__hub__claim` — atomically lock (or queue for) a feature or file set so no
  peer touches it concurrently.
- `mcp__hub__release` — relinquish a held claim, or cancel a queued ticket.
- `mcp__hub__post` — send a durable message to a peer's `agent_id`, to `*`
  (broadcast), or to `topic:<name>`.
- `mcp__hub__inbox` — peek pending messages without consuming them.
- `mcp__hub__read` — advance your read cursor to acknowledge what you have consumed.

## The protocol

1. **On start — register and state your goal.** `mcp__hub__register`, then
   `mcp__hub__post` your **goal**: one line naming the objective you were dispatched
   for and your done-condition. `roster` + `claims` to see who is live and what they
   hold, then `claim` what you are about to touch. If a live peer already holds it,
   `post` a deferred-interest note to them and to `*`, then stand down — never
   collide, never silently take over.
2. **While working:** `post` a short note at each stage transition so peers and the
   driver can see progress, and `inbox` + `read` to pick up messages directed at
   you, broadcasts, and topics you follow.
3. **On finish — report, then release.** `post` a **report**: your goal, what you
   did, the result (pass/fail + evidence), and any follow-ups or orphan bugs you
   found. This is the report the driver and your SubagentStop hook expect. Then
   `release` every claim you hold.

Always open with a goal and close with a report — a peer or the driver reading only
your first and last hub posts must understand what you set out to do and what
happened.

## Two channels

- **hub** is the durable state-and-coordination channel: claims, intent and
  interest, goal, stage posts, and the final report survive even your death, so the
  driver can reconcile them onto the ledger.
- **SendMessage** is the live, context-preserving back-channel the operator or
  driver uses to steer you mid-run (the `redo` path) without restarting you from zero.

## Role scope

- **Stage subagent** (dispatched for one unit of work): post your goal, coordinate,
  and post your report, but do NOT write the `/.machine/sessions/` ledger or
  orchestrate peers.
- **Factory-job agent** (owns one feature end to end): run the full handshake, post
  every stage transition, report and `release` on close — but never orchestrate a fleet.
- **Driver** (main loop / drill): claims a feature before dispatch, reconciles peer
  goals, posts, and reports onto the ledger, and releases on `merged` or `dropped`.
