---
name: taskboard
description: Use when the user says "taskboard", "taskboard board", "kanban board", or "drill board" — explains the local taskboard kanban workflow, board-per-cwd identity, the stage-to-column mapping, and the read-update-consult discipline that mirrors the drill ledger onto a card per feature.
---

# taskboard — local kanban board for the drill ledger

`taskboard` (github.com/tcarac/taskboard) is a local, keyless kanban server. One
`taskboard` daemon serves every board over a single SQLite database; the board for
this repo is one taskboard **project**, selected by the current working directory
(board-per-cwd). The drill uses it to mirror the live `/.machine/sessions/` ledger:
one card per in-flight feature, moved across columns as the feature crosses stages.

The board is a projection of the ledger, not a second source of truth. The ledger
under `/.machine/sessions/` remains the durable record; the board is the at-a-glance
view. When the two disagree, the ledger wins and the board is re-synced.

Two surfaces share one database: the MCP server (`taskboard mcp`, wired into
`plugin.json`) for card operations, and the web UI (`taskboard start`, default
`http://localhost:3010`) for a human view. Card operations work through MCP even
when the web daemon is down. The shared SQLite database runs in WAL mode, so the
web UI and the MCP server can write concurrently without lock contention.

## Provisioning

`scripts/bootstrap.sh` installs taskboard prebuilt-first: it downloads the pinned
release binary for the host platform (a statically-linked binary with the web
frontend embedded), so the common path needs neither Go nor Node. A source build is
attempted only if the download fails and Go 1.24 plus Node 22 are present (the
embedded frontend makes Node a build-time dependency). When neither path is
available, bootstrap warns and skips — taskboard is an addon, so a failed install
never aborts the run; board projection is simply off for that session. The pinned
release version is a constant in `bootstrap.sh`; bump it deliberately to upgrade.

## Rollback

To remove this addon: delete the `taskboard` entry from `.claude-plugin/plugin.json`
`mcpServers`, delete `mine/skills/taskboard/`, and optionally run `taskboard stop`.
Leave the SQLite database at `~/.config/taskboard/taskboard.db` untouched.

## IDs — always load from taskboard.json

Never hardcode a `projectId`. A taskboard project id is a server-generated ULID; it
is not derivable from the cwd. Read `.machine/taskboard.json` at the start of every
board operation — it is the single source of truth for this repo's project id, prefix,
and board URL.

`.machine/taskboard.json` schema (per-repo runtime state, resolved on first use; not
committed):

```
version    schema version (1)
cwd        absolute working directory this project belongs to
name       basename of cwd — the project name
prefix     "P" + first 6 upper hex chars of sha1(absolute cwd) — the lookup key
projectId  server-generated ULID — required by every ticket/board call
url        web board URL (http://localhost:3010)
resolvedAt ISO-8601 timestamp of resolution
```

## Resolve or provision the project

When `.machine/taskboard.json` is missing or stale, resolve the project before any
card work. Two equivalent paths:

- Deterministic helper: run `scripts/taskboard-resolve.sh`. It computes the name and
  prefix, lists projects, matches on prefix, creates the project if absent, and writes
  `.machine/taskboard.json`. Use this when a plain shell path is wanted.
- MCP path (richer disambiguation): call `list_projects` and match a project whose
  `prefix` equals this repo's prefix AND whose `description` carries this repo's
  absolute cwd. On zero matches, `create_project` with the name, the prefix, and the
  absolute cwd written into `description` (the canonical remote disambiguator). On
  exactly one match, use it. On more than one match, do not guess — require a persisted
  `projectId` or create with a more specific name. Persist the returned id into
  `.machine/taskboard.json`.

The persisted `cwd` guards the local file against a prefix collision; the project
`description` is the canonical remote disambiguator across projects.

## Three columns

taskboard has exactly three fixed status columns; there are no custom columns. The
status enum is `todo`, `in_progress`, `done`.

## Stage-to-column mapping

The drill projects each ledger entry's lifecycle stage onto one card. This table is
the single source of truth for that mapping.

| Drill status | taskboard column |
|---|---|
| grilling, planning, plan-review, plan-ready | `todo` |
| implementing, arbiter | `in_progress` |
| merge-proposed | `done` |
| merged, dropped | card removed (`delete_ticket`) |

Card title convention: `[<status>] <label>`. The card body carries the entry id, the
current stage, the git branch, and a one-line summary. On a stage transition,
`move_ticket` to the target column and `update_ticket` to retitle, as one action. On
`merged` or `dropped`, `delete_ticket` so the board converges to empty — the
`/.machine/sessions/` ledger remains the history.

## Read-update-consult discipline

Read the board before starting work on a feature, update the card the moment a
feature changes state, and consult it when deciding what is in flight. Do not batch
card updates to the end of a session. Every state change that touches a feature moves
its card and refreshes its title in the same action; no silent edits, no card left in
the wrong column.

## Board trust

Only the drill (the user-facing driver session) writes cards, exactly as only the
drill writes the `/.machine/sessions/` ledger. A dispatched sub-agent reports stage
transitions through `mesh`; the drill reconciles those onto the ledger and projects
them onto the board on its own turn. A card the drill did not create this session is
untrusted and is surfaced for human review rather than acted on.

## Operating tools (taskboard MCP)

Real tool names and required arguments, from the taskboard MCP server. Tools are
deferred — load via `ToolSearch` with `select:<name>` first.

- Projects: `list_projects` (status?), `get_project` (id), `create_project` (name,
  prefix; description?, icon?, color?), `update_project` (id; name?, prefix?,
  description?, status?), `delete_project` (id).
- Tickets (cards): `list_tickets` (projectId?, teamId?, status?, priority?),
  `get_ticket` (id), `create_ticket` (projectId, title; description?, status?,
  priority?, teamId?, dueDate?), `update_ticket` (id; title?, description?, status?,
  priority?, …), `move_ticket` (id, status), `delete_ticket` (id).
- Board: `get_board` (projectId?) — returns the board grouped by the three columns.

Card statuses and any status filter must use the exact enum `todo`, `in_progress`,
`done`. Teams and subtasks exist upstream but are out of scope for the drill
projection.
