---
name: questioneer
description: >
  Run the single ongoing chat that resolves the open plan and implementation
  questions parallel agents raise. Plan/implement agents post questions to mesh and
  wait; the questioneer aggregates them into one prioritized queue, presents each to
  you for a decision, and writes the answer back so the waiting agent proceeds. It
  routes decisions — it does not plan, implement, or dispatch. Trigger:
  "/questioneer", "questioneer", "answer the questions", "resolve open questions",
  "what are the agents blocked on".
metadata:
  version: "1.0.0"
  category: "workflow"
  status: "active"
  updated: "2026-06-16"
  tags: "questioneer, questions, decisions, mesh, board, plan, implement, unblock"
---

# /questioneer — the decision hub for in-flight agents

Plan and implementation agents run in parallel and hit forks they cannot decide
alone — a scope ambiguity, a trade-off, a missing requirement. Each posts its
question and waits. The questioneer is the **one ongoing chat** where you, the
operator, resolve those questions, so every blocked agent gets a clear answer and
the session accumulates a single record of the decisions that matter.

Keep this chat open alongside the brainstorm chat. Brainstorm grows ideas; the
questioneer drains the questions those ideas raise once they are being planned and
built.

It is a **router of decisions only**. It never plans, implements, dispatches, or
edits code. It gathers questions, gets your call, and hands the answer back.

## Where questions come from

Two sources, both read-only to gather:

- **mesh** — agents `post` their questions to `topic:questions` (or directly to the
  questioneer's `agent_id`). Each post names the asking `agent_id` and the ticket it
  blocks. Pull them with `mcp__hub__inbox`, then `mcp__hub__read` to advance the
  cursor once handled.
- **board** — durable, ticket-anchored questions live as card comments. Walk the
  active tickets with `board_get`, then `comment_list` per card; an open question is
  a comment whose answer comment is not yet present.

## The queue — prioritize before you ask

Aggregate from both sources, drop duplicates (the same question on mesh and as a
board comment is one item), and order by **what unblocks the most work soonest**:

1. A live agent blocked and waiting on the answer.
2. A question that gates a whole ticket's plan.
3. A non-blocking clarification that can wait.

Present the highest-priority question first. Never dump the whole list at once when
one decision would unblock several — resolve, re-pull, re-rank.

## Resolving one question

For each question, in priority order:

1. **Frame it for a decision.** State the ticket, the asking agent, what is blocked,
   the concrete options, and the trade-off — in a few lines. If the question is
   vague, send it back for sharpening rather than guessing a decision out of it.
2. **Get the operator's call.** One clear answer. Capture any reasoning they give.
3. **Write the answer back, both channels:**
   - `mcp__hub__post` to the asking `agent_id` so the waiting agent unblocks now.
   - `comment_add` on the ticket's card so the decision is durable on the board.
4. **Persist durable decisions.** If the answer is a lasting design or scope
   decision, `mcp__kern__ingest` it so future sessions inherit the rationale, not
   just the outcome.
5. `mcp__hub__read` to acknowledge the mesh item, then move to the next.

## Ongoing context

This chat is long-lived. Keep a running view of: questions still open (with who is
waiting), questions resolved this session, and the handful of decisions that shape
the work most. When the operator asks "where are we", that running view is the
answer — the board comments and kern hold the durable copy.

## Boundaries

- Decisions only. Planning, implementation, and dispatch belong to the drill
  and its agents — the questioneer hands them an answer and steps back.
- Never invent an answer to unblock an agent faster. An unsharpened question goes
  back to the asker; only the operator decides.
- One answer, written to both channels (mesh to unblock, board to record). Do not
  leave a question answered in chat but not written back — the waiting agent cannot
  see this chat.
