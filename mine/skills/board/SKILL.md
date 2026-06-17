---
name: board
description: Use when the user says "board", "kanban board", or "drill board" — explains the local board kanban workflow, board-per-cwd identity, the stage-to-column mapping, labels/checklists/due-dates, and the read-update-consult discipline that mirrors the drill ledger onto a card per task, idea, or major step.
---

# board — local kanban served by hub

`board` is the machine's kanban surface, absorbed into the `hub` singleton daemon
(a2). Hub exposes all 21 board verbs via MCP under the `mcp__plugin_machine_hub__`
prefix, backed by one repo-scoped JSON state file under `.board/`. The data model is
projects -> columns -> cards -> comments, where a card also carries tags, an
assignee, a due date, and named checklists (todo groups). On top sit a label color
registry (`label_set`/`label_list`/`label_delete`) and `card_find` for querying cards
by label. The driver uses the board as its living work surface and to mirror the live
`/.machine/sessions/` ledger: a card per task, idea, or major step in flight, moved
across columns as it changes state.

The board is a projection of the ledger, not a second source of truth. The ledger
under `/.machine/sessions/` remains the durable record; the board is the at-a-glance
view. When the two disagree, the ledger wins and the board is re-synced.

Hub serves the board on `http://localhost:7777` (WebSocket live view at `/ws`,
REST API at `/api/<verb>`). Card operations work through MCP even when the web UI
is not open. All board writes are serialized by the same mkdir-lock used for mesh
state, so concurrent MCP calls and web drag-and-drop cannot corrupt `.board/state.json`.

## Provisioning

Hub ships in-plugin as a Rust binary (`hub/target/release/hub`). Bootstrap builds it
with `cargo build --release --manifest-path hub/Cargo.toml`. The SessionStart hook
`hub-ensure.sh` starts `hub serve` on `:7777` and polls health up to 5 seconds. An
already-running daemon is a no-op. Hub is the single process for both mesh and board
— no separate board daemon.

## Rollback

To remove board support: drop the board verb calls from skills and agents. The hub
binary continues serving mesh verbs. The `.board/` state directory (gitignored) can
be deleted to discard board data.

## IDs — always load from board.json

Never hardcode a `projectId`. A board project id is a server-generated ULID; it is
not derivable from the cwd. Read `.machine/board.json` at the start of every board
operation — it is the single source of truth for this repo's project id, name, and
board URL.

`.machine/board.json` schema (per-repo runtime state, resolved on first use; not
committed):

```
version    schema version (1)
cwd        absolute working directory this project belongs to
name       basename of cwd — the project name
projectId  server-generated ULID — required by board_get and every card call
url        web board URL (http://localhost:7777)
resolvedAt ISO-8601 timestamp of resolution
```

## Setting up a project

A board is one board **project** bound to this repo's cwd (board-per-cwd:
name = basename(cwd)). Set it up once; every later board call reads its id from
`.machine/board.json`. Resolve the project whenever `.machine/board.json` is missing
or stale, before any card work:

1. Call `project_resolve` with `name` = basename of the repo cwd. It is get-or-create
   by name: an existing project with that name is returned, otherwise a new one is
   created. The returned `project.id` is the ULID to persist.
2. Write `.machine/board.json` with the schema above (`version` 1, the absolute
   `cwd`, the `name`, the returned `projectId`, `url` `http://localhost:7777`, and the
   current ISO-8601 `resolvedAt`).
3. Create the six lifecycle columns (below) once via `column_create`, left-to-right
   in order, if the project has none yet.

### Viewing the board

Open `http://localhost:7777/` in a browser. The page connects via WebSocket and
live-reloads on every card or roster mutation. To start hub manually:
`hub serve` (default `http://localhost:7777`).

## Six lifecycle columns

The drill creates ITS project with six fixed lifecycle columns via `column_create`,
left-to-right exactly: **Gathered | Approved | In Progress | Mergable | To Merge |
Merged**. The board IS the pipeline view: the agent gathers a task -> the user
approves it -> a subagent plans, reviews, and implements -> the build goes mergable ->
the user approves the merge -> it merges into master. This table is the single source
of truth for the drill-stage -> column mapping.

| Drill status | board column |
|---|---|
| grilling | `Gathered` |
| planning, plan-review, plan-ready | `Approved` |
| implementing | `In Progress` |
| arbiter, merge-proposed | `Mergable` |
| (user approved merge; resolve/merge in flight) | `To Merge` |
| merged | `Merged` (card KEPT, not deleted) |
| dropped | card removed (`card_delete`) |

Card title convention: `[<status>] <label>`. The card `body` carries the entry id,
the current stage, the git branch, and a one-line summary. On a stage transition,
`card_move` the card to the target column and `card_update` to retitle, as one action.
On `merged`, `card_move` the card to the `Merged` column and keep it there — the
Merged column is the completed-pipeline record. Only `dropped` calls `card_delete`
(an abandoned feature leaves no card); the `/.machine/sessions/` ledger remains the
durable history in both cases.

## Read-update-consult discipline

Read the board before starting work on a feature, update the card the moment a
feature changes state, and consult it when deciding what is in flight. Do not batch
card updates to the end of a session. Every state change that touches a feature moves
its card and refreshes its title in the same action; no silent edits, no card left in
the wrong column.

## Board trust

Only the drill (the user-facing driver session) writes cards, exactly as only the
drill writes the `/.machine/sessions/` ledger. A dispatched subagent reports stage
transitions through `mesh`; the drill reconciles those onto the ledger and projects
them onto the board on its own turn. A card the drill did not create this session is
untrusted and is surfaced for human review rather than acted on.

## Operating tools (hub MCP — board verbs)

All board verbs are served by hub at `http://localhost:7777/mcp` under the prefix
`mcp__plugin_machine_hub__`. Tools are deferred — load via `ToolSearch` with
`select:<name>` first.

- Projects: `project_resolve` (name) — get-or-create by name; `project_list`.
- Board read: `board_get` (projectId) — the project plus its columns left-to-right,
  each carrying its cards by sort with a `commentCount`.
- Columns: `column_create` (projectId, name), `column_update` (id; name) — rename,
  `column_delete` (id) — delete cascades to the column's cards and their comments.
- Cards: `card_create` (columnId, title; body?, tags?, assignee?, due?),
  `card_update` (id; title?, body?, tags?, assignee?, due?) — `assignee`/`due` accept
  null to clear, `tags` replaces the whole list; `card_move` (id, toColumnId;
  newIndex?) — 0-based slot in the destination; `card_delete` (id) — also deletes the
  card's comments; `card_find` (tag; projectId?) — cards carrying a tag/label.
- Labels: `label_set` (name, color), `label_list`, `label_delete` (name) — the color
  registry behind tag chips; an unregistered tag falls back to an auto color.
- Checklists: `checklist_add` (cardId; title?), `checklist_remove` (cardId,
  checklistId); items: `checkitem_add` (cardId, checklistId, text), `checkitem_set`
  (cardId, checklistId, itemId; done?, text?) — toggle/edit, `checkitem_remove`
  (cardId, checklistId, itemId).
- Comments: `comment_add` (cardId, author, body), `comment_list` (cardId) —
  oldest-first. A comment on a card with an `assignee` posts a `board:comment` mesh
  message to that agent.

Every mutation bumps `state.rev` and pushes a full state snapshot over the WebSocket
`/ws` channel so open browser tabs re-render immediately.
