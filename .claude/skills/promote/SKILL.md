---
name: promote
description: >
  Promote crystallized brainstorm findings into board tickets. After a brainstorm
  (drill grill) settles one or more well-scoped features, this turns each into a
  card on the current cwd's board so the drill's plan step can iterate per ticket.
  The bridge between brainstorm and plan — it creates tickets, it does not plan or
  implement. Trigger: "/promote", "promote these", "promote to tickets", "make
  tickets", "turn findings into cards".
metadata:
  version: "1.0.0"
  category: "workflow"
  status: "active"
  updated: "2026-06-16"
  tags: "promote, brainstorm, board, ticket, card, drill, plan, intake"
---

# /promote — brainstorm findings into board tickets

The brainstorm chat produces ideas; the board holds work. `/promote` is the bridge:
it takes the crystallized findings of a brainstorm and writes one ticket per feature
onto the current repo's board, in the intake lane, ready for the drill's plan step.

It is **write-only into the board**. It does not plan, dispatch, or implement — it
captures agreed scope as tickets. Planning each ticket is the next step (drill).

## When to run

Run it once a brainstorm has crystallized — every finding has a non-vague **What**
(the specific feature/problem), **How** (rough direction), and **Why now** (agreed
worth doing), the same bar Brainstorm Mode uses before it calls something a task.

Do **not** promote vague ideas, half-formed directions, or a single feature split
into many fragments. One ticket = one coherent, independently-plannable feature.

## What a ticket carries

Each promoted finding becomes one board card:

- **title** — a concise feature name (the *What*), no stage prefix.
- **body** — the agreed scope in a few lines: *What* (one sentence), *How* (rough
  direction), *Why now*, and any **Decisions made** during the brainstorm. This is
  the seed the plan step expands — keep it factual, not prose.

## The board surface

The board is project-per-cwd. Use the `board` MCP verbs (the stable contract);
never write `.board/` state files directly.

1. **Resolve the project.** Read `/.machine/board.json` for this repo's cached
   `projectId`. If absent, call `project_resolve` with `name` = the repo root's
   basename to get-or-create it.
2. **Find the intake lane.** Call `board_get` for the project. The intake lane is
   the leftmost column (lowest `sort`). If the board has no columns yet, create one
   named `Backlog` with `column_create`, and use it.
3. **De-dup before create.** If a card with the same title already sits in intake,
   update its body with `card_update` instead of creating a near-duplicate — single
   source of truth applies to tickets too.
4. **Create the card.** Call `card_create` with `columnId` = the intake lane,
   `title`, and `body` as above. One call per finding.

## How to run

1. Gather the crystallized findings — from the brainstorm conversation, or from the
   list passed as the skill argument. Confirm each clears the What/How/Why-now bar;
   drop or flag any that do not.
2. Resolve the project and intake lane (above).
3. For each finding: de-dup, then create (or update) its card.
4. Report back: list each ticket's title and id, and name the next step — the drill
   plans each ticket in turn, repeating until you approve. Do not start planning
   here; `/promote` ends at tickets on the board.

## Boundaries

- Promote ends at the board. Planning, dispatch, and implementation are the
  drill's job, one ticket at a time.
- Questions raised while promoting (scope still fuzzy on a finding) go back to the
  brainstorm/questioneer, not into a half-baked ticket.
- Never delete or reorder existing tickets the user did not ask you to touch.
