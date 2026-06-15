---
name: trello
description: Use when the user says "trello", "board", or "kanban", or when starting/finishing any task that should be tracked — explains the board workflow, list semantics, and the read-update-consult discipline.
---

# Trello — kern board workflow

Board: **Kern** — <https://trello.com/b/IGAdVSqg/kern> (shortLink `IGAdVSqg`,
full id `6a20b8c43bdcf4af3f539ba4`).

The planning board is the source of truth for what to work on, what to keep in
mind, and what's been decided. **Read it before starting work, update it as you
work, consult it when making decisions.** Don't batch updates to the end.

> **Touch a ticket = update its state + column.** Any time work touches a card —
> start, progress, pause, finish — move it to the matching list AND keep its
> priority label current in the same action. No silent edits, no leaving a worked
> card in TODO. Pulled → DOING; shipped → DONE; blocked → HOLD; needs decision
> → (RE)EVALUATE ((RE)EVALUATE also takes in-flight cards bounced back for
> reevaluation, like a soft block).

## IDs — always load from trello.json

**Never hardcode IDs.** Read `.machine/trello.json` at the start of
every Trello operation. The JSON is the single source of truth for board ID, list
IDs, and label IDs.

```
board.id          → full board ID (required by move_card)
board.shortLink   → shortLink (usable by get_card / add_comment)
board.url         → https://trello.com/b/IGAdVSqg/kern
lists.*           → semantic list → ID mapping
labels.priorities.* → priority label IDs (this board has NO category labels)
```

## Board setup (when the board changes)

When `board.id` / `board.shortLink` changes, or `trello.json` is empty/missing,
run this setup sequence to repopulate the JSON:

1. **Set active board** — `mcp__trello__set_active_board` with
   `https://trello.com/b/IGAdVSqg/kern` (or the board id).
2. **Fetch lists** — `mcp__trello__get_lists` → map each list by name to its ID.
   Match list names to these semantic keys (kern board names):

   | Key | Expected list name (case-insensitive) |
   |-----|---------------------------------------|
   | `evaluate` | (RE)EVALUATE |
   | `reminder` | REMIND |
   | `onHold`   | HOLD |
   | `todo`     | TODO |
   | `inDoing`  | DOING |
   | `done`     | DONE |

3. **Fetch labels** — `mcp__trello__get_board_labels` → map by name. This board
   carries **priority labels only** (no category labels):

   | Key path | Expected label name | Color |
   |----------|---------------------|-------|
   | `priorities.high`     | High     | red    |
   | `priorities.medium`   | Medium   | orange |
   | `priorities.low`      | Low      | yellow |

4. **Write** the populated structure back to `trello.json` using the Write tool.

If a list or label name doesn't match exactly, ask the user which name maps to
which semantic key before writing.

## List semantics

| Semantic key | Meaning |
|---|---|
| `evaluate` | Unvetted ideas / proposals, AND worked cards sent back for reevaluation. |
| `reminder` | Constraints + gotchas. Read before designing. Not tasks. |
| `onHold`   | Paused / blocked work. |
| `todo`     | Committed, not started. |
| `inDoing`  | Actively being worked. |
| `done`     | Complete. |

TODO → DOING → DONE is the default kanban flow.

## Labels

Each card carries **one priority label only** — this board has no category
labels.

**`update_card_details` `labels` REPLACES the full array** — pass the priority
ID when updating so it isn't dropped. Same for new cards via `add_card_to_list`.

Priority heuristic (kern): security holes / retrieval-quality / "can't measure"
blockers = High; subsystem improvements = Medium; refinements / cleanups = Low.

## Workflow

- **Before work** — read `reminder` list (constraints to honor) and check
  `inDoing`/`todo` for the relevant card. If no card exists for non-trivial
  work, make one in `todo`.
- **Idea, not committed** → card in `evaluate`. Don't start building it.
- **Constraint / gotcha** → card in `reminder` (or comment on the existing one).
- **Starting a task** → move card to `inDoing`, add a comment on what you're doing.
- **Decision / blocker mid-task** → add a comment on the card. New work
  discovered → add a card so the board stays the source of truth.
- **Finished** → comment the outcome, move card to `done`.

## Operating the board (MCP tools)

Tools are deferred — load via `ToolSearch` with `select:<name>` first. Common:

- Read: `mcp__trello__get_lists`, `mcp__trello__get_cards_by_list_id`,
  `mcp__trello__get_card`, `mcp__trello__get_card_comments`,
  `mcp__trello__get_board_labels`.
- Write: `mcp__trello__add_card_to_list`, `mcp__trello__add_comment`,
  `mcp__trello__move_card`, `mcp__trello__update_card_details`.
- Setup: `mcp__trello__set_active_board`.

**`move_card` gotcha:** it 400s on shortLinks — pass `board.id` (full) and the
full card ID. `get_card` / `add_comment` accept shortLinks fine. Fetch the card
first to get its full ID, then move.
