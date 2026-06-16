---
name: board
description: Use when the user says "board", "kanban board", or "drill board" — explains the local board kanban workflow, board-per-cwd identity, the stage-to-column mapping, and the read-update-consult discipline that mirrors the drill ledger onto a card per feature.
---

# board — local kanban for the drill ledger

`board` is the machine's own zero-dependency kanban addon: a single Node file
(`board/board.mjs`, built mesh-style) that exposes an MCP verb surface over stdio
plus a CDN-styled web UI with an SSE live-update channel on `http://localhost:3010`,
backed by one repo-scoped JSON state file under `.board/`. The data model is
projects -> columns -> cards -> comments. The drill uses it to mirror the live
`/.machine/sessions/` ledger: one card per in-flight feature, moved across columns as
the feature crosses stages.

The board is a projection of the ledger, not a second source of truth. The ledger
under `/.machine/sessions/` remains the durable record; the board is the at-a-glance
view. When the two disagree, the ledger wins and the board is re-synced.

Two surfaces share one store: the MCP server (wired into `plugin.json` as a stdio
entry, `node ${CLAUDE_PLUGIN_ROOT}/board/board.mjs mcp`, exactly like mesh) for card
operations, and the web UI (`node board/board.mjs serve`, default
`http://localhost:3010`) for a human view. Card operations work through MCP even when
the web daemon is down — the stdio surface is independent of the web port. A single
mkdir-lock around every mutation serializes web-UI writes and MCP writes to one
`state.json`, so the drill and a human dragging cards cannot corrupt it.

## Provisioning

board ships in-plugin: it is a single zero-dependency Node file, so there is nothing
to download or build — it needs only Node (>=22.5.0, already required by the machine
for context-mode). `scripts/bootstrap.sh` starts its web daemon (`start_board`):
`node board/board.mjs serve` on `:3010`, polling `curl http://localhost:3010/api/board`
until ready; an already-running daemon is treated as ok. When Node is missing or the
daemon fails to bind, bootstrap warns and skips — board is an addon, so it never
aborts the run; the web view is simply off for that session and MCP card ops still
work. There is no separate install step, no Go, no SQLite, no prebuilt download.

## Rollback

To remove this addon: delete the `board` entry from `.claude-plugin/plugin.json`
`mcpServers`, delete `mine/skills/board/` and `board/`, and optionally run
`node board/board.mjs stop`. Drop the `start_board` call from
`scripts/bootstrap.sh`. The repo-scoped `.board/` state directory (gitignored) can be
deleted to discard board data.

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
url        web board URL (http://localhost:3010)
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
   `cwd`, the `name`, the returned `projectId`, `url` `http://localhost:3010`, and the
   current ISO-8601 `resolvedAt`).
3. Create the five lifecycle columns (below) once via `column_create` if the project
   has none yet.

Resolution is a single MCP call plus a small file write — there is no shell resolver
and no sha1-prefix scheme. The persisted `cwd` guards the local file against reuse in
a different working directory.

### Viewing the board

The web UI is started by bootstrap (`start_board`). To start it by hand, run
`node board/board.mjs serve` (default `http://localhost:3010`); card operations work
without it. The page live-reloads via SSE: a card moved in one tab appears in another
within about a second.

## Five lifecycle columns

The drill creates ITS project with five lifecycle columns via `column_create`, in
left-to-right order. This table is the single source of truth for the
drill-stage -> column mapping.

| Drill status | board column |
|---|---|
| drilling | `drilling` |
| planning, plan-review | `planning` |
| plan-ready | `plan-ready` |
| implementing, arbiter | `implementing` |
| merge-proposed | `merge-proposed` |
| merged, dropped | card removed (`card_delete`) |

Card title convention: `[<status>] <label>`. The card `body` carries the entry id,
the current stage, the git branch, and a one-line summary. On a stage transition,
`card_move` the card to the target column and `card_update` to retitle, as one action.
On `merged` or `dropped`, `card_delete` so the board converges to empty — the
`/.machine/sessions/` ledger remains the history.

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

## Operating tools (board MCP)

Real tool names and required arguments, from the board MCP server. Tools are deferred
— load via `ToolSearch` with `select:<name>` first.

- Projects: `project_resolve` (name) — get-or-create by name; `project_list`.
- Board read: `board_get` (projectId) — the project plus its columns left-to-right,
  each carrying its cards by sort with a `commentCount`.
- Columns: `column_create` (projectId, name), `column_delete` (id) — delete cascades
  to the column's cards and their comments.
- Cards: `card_create` (columnId, title; body?), `card_update` (id; title?, body?),
  `card_move` (id, toColumnId; newIndex?) — 0-based slot in the destination,
  `card_delete` (id) — also deletes the card's comments.
- Comments: `comment_add` (cardId, author, body), `comment_list` (cardId) —
  oldest-first.

Every mutation bumps `state.rev` and pushes `data:{"rev":N}` over the SSE `/events`
channel so open web tabs refetch.
