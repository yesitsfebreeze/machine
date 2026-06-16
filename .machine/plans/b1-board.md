# Implementation Plan — `board`: a single-file mesh-style kanban (b1-board)

Direction chosen by the user: **A** — single zero-dependency `board/board.mjs` built
like `mesh/mesh.mjs`, using LocalBoards only as a DESIGN TEMPLATE (data model + event
names), copying none of its code. Replaces the taskboard addon entirely.

## 0. Why not vendor LocalBoards (verified)
LocalBoards (github.com/florian-strasser/LocalBoards @ master, MIT) is NOT
SQLite-capable: it is hard-wired to **MySQL** (`mysql2/promise`, `createPool`,
MySQL-only DDL — `ENGINE=InnoDB`, `AUTO_INCREMENT`, `utf8mb4_0900_ai_ci`), with no DB
abstraction. It is a full **Nuxt 4 SSR app** (TipTap, Tailwind, socket.io, i18n; 20
deps) needing `npm install` + `nuxt build`. Vendoring it would force a MySQL server +
node build into the machine and fights every machine law (zero-dep, "vendor like
mesh", single JSON state, ships as a Markdown payload). Hence the mesh-style rewrite.

## 1. Template (not code) borrowed from LocalBoards
Only the shape — original code, so no LICENSE-vendoring obligation; keep a courtesy
`board/ATTRIBUTION.md` line (MIT design credit).
- Data model flattened from its MySQL tables: projects -> columns (its "areas") ->
  cards -> comments. Drop users/invitations/notifications/sessions/api-keys/attachments.
- Realtime event names from its `server/plugins/socket.io.ts`
  (addCard/updateCard/movedCard/deletedCard/addColumn/updateColumn/deleteColumn/addComment)
  collapsed to one coarse `state`/`{rev}` push in v1.
- Kanban UX: columns left-to-right, drag cards between columns, click card -> comments.

## 2. The single file — `board/board.mjs` (~600-800 lines, structured like mesh.mjs)
2.1 STORAGE (copied pattern from mesh): repo-scoped `dataDir()` via git common-dir
(override `BOARD_DIR`), under `.board/` (gitignored). One `state.json`, one `.lock`
mkdir mutex, atomic `renameSync`. Reuse mesh's `#lock`/`#unlock`/`#txn`/`#load`/`#save`
and its monotonic `ulid()` for ids.
  state.json: `{ projects:{<pid>:{id,name,createdAt}}, columns:{<cid>:{id,projectId,name,sort}}, cards:{<kid>:{id,columnId,title,body,sort,createdAt}}, comments:{<mid>:{id,cardId,author,body,createdAt}}, rev:<int> }`
2.2 DOMAIN VERBS (the MCP + HTTP surface — snake_case like mesh):
  - `project_resolve(name)` get-or-create by name (board-per-cwd = basename(cwd)); `project_list`
  - `board_get(projectId)` -> project + columns + cards + comment counts, grouped (the one read UI/drill need)
  - `column_create(projectId,name)`, `column_delete(id)`
  - `card_create(columnId,title,body?)`, `card_update(id,title?,body?)`, `card_move(id,toColumnId,newIndex)`, `card_delete(id)`
  - `comment_add(cardId,author,body)`, `comment_list(cardId)`
  Each mutation bumps `state.rev` and triggers a live push.
2.3 MCP SERVER — STDIO (like mesh/kern), reusing mesh's TOOLS table / `toolsList()` /
`dispatch()` / `handleLine()` / `serveStdio()` almost verbatim; `serverInfo.name="board"`.
Run via `board mcp`. Stdio over http: machine's gold transport, no port, self-heals;
card ops stay independent of the web daemon (the guarantee taskboard gave).
2.4 WEB UI + LIVE UPDATES (the new part): `board serve` -> Node built-in `http` on port
3010 (reclaimed from taskboard).
  Routes: `GET /` -> one self-contained page (sibling `board/web/index.html`, zero-dep);
  `GET /api/board?projectId=` -> board_get JSON; `POST /api/<verb>` -> thin JSON wrappers
  over the domain verbs; LIVE CHANNEL = SSE `GET /events` (text/event-stream) — on any
  mutation write `data:{"rev":N}\n\n` to all clients; page refetches /api/board.
  CDN STYLING: page pulls one CSS framework via `<link>` — Pico.css
  (cdn.jsdelivr.net/npm/@picocss/pico@2, classless, tiny); Tailwind Play CDN as alt.
  Drag: native HTML5 DnD (zero deps) with SortableJS CDN `<script>` as fallback. Client
  JS vanilla inline + `EventSource('/events')`.
2.5 CLI (copied from mesh): `board mcp` | `board serve` | `board stop` (pidfile) | `board --version`.

## 3. Integration (mirrors mesh + old taskboard)
3.1 `.claude-plugin/plugin.json`: replace the `taskboard` mcpServers entry with a STDIO
entry shaped like mesh:
  `"board": { "command": "node", "args": ["${CLAUDE_PLUGIN_ROOT}/board/board.mjs", "mcp"] }`
  No launcher script (pure Node like mesh). Bump plugin version.
3.2 `scripts/bootstrap.sh`: replace `install_taskboard()` + helpers (taskboard_ready,
taskboard_build_source, go_ok, the `. taskboard/install.sh` source, all TASKBOARD_*
vars) with a tiny `start_board()`: no install (ships in-plugin, needs only Node via
`ensure_node`); `node board/board.mjs serve` daemonized on :3010; poll
`curl http://localhost:3010/api/board` until ready; "already running" pidfile = ok;
warn-and-skip on failure (addon contract preserved). Strictly less code; no Go/MySQL/download.
3.3 Per-repo state -> `.machine/board.json` (replaces `.machine/taskboard.json`):
`{version,cwd,name(basename),projectId(ulid),url(http://localhost:3010),resolvedAt}`.
DELETE `scripts/taskboard-resolve.sh` — resolution is now one MCP call
`project_resolve(name)` then the drill writes board.json. No sha1-prefix scheme.

## 4. Drill rewiring
4.1 Stage->column mapping (new single source of truth, in `mine/skills/board/SKILL.md`).
Drill creates ITS project with 6 fixed lifecycle columns via `column_create`,
left-to-right: **Gathered | Approved | In Progress | Mergable | To Merge | Merged**.
This board IS the pipeline view: agent gathers a task -> user approves it -> subagent
plans/reviews/implements -> build goes mergable -> user approves the merge -> it merges
into master. Drill status -> column:
  grilling -> Gathered | planning,plan-review,plan-ready -> Approved |
  implementing -> In Progress | arbiter,merge-proposed -> Mergable |
  (user approved merge, resolve/merge in flight) -> To Merge |
  merged -> Merged (card KEPT in the Merged column, not deleted) |
  dropped -> card removed (`card_delete`)
  Card title `[<status>] <label>`; body = entry id/stage/branch/summary. Transition =
  `card_move` + `card_update` in one action. Read-update-consult + board-trust (only
  drill writes cards) unchanged.
4.2 Files to rewrite (verbs project_*/column_*/card_*/comment_*, stdio MCP):
  - REPLACE `mine/skills/taskboard/` -> new `mine/skills/board/SKILL.md`: rename/triggers;
    "one zero-dep Node daemon, stdio MCP + web UI/SSE on :3010"; provisioning = ships
    in-plugin, needs only Node, started by bootstrap (delete prebuilt/Go/SQLite prose);
    IDs from `.machine/board.json`; the 5-column mapping; new verb table; rollback =
    remove board plugin.json entry, delete `board/` + `mine/skills/board/`, `board stop`.
  - `.claude/skills/drill/SKILL.md` (~L245-247): reference `@mine/skills/board/SKILL.md`,
    "a board card", `card_delete` on merge/drop.
  - `.claude/skills/drill/references/assemble.md` (~L30 addon-table row, ~L174
    local-services line): describe the board (zero-dep Node, no Go/MySQL, skips only if
    Node missing).
  - `.claude/skills/oil/SKILL.md` L33: `.machine/taskboard.json` -> `.machine/board.json`.
  - `.claude/agents/default.md` (~L122, ~L177): taskboard->board in addon list + the
    "MUST NOT write ... the board" guard.
  - `.claude/rules/coding-standards.md` (~L82): remove the inaccurate "used by the
    taskboard to wake at the soonest pending fire_at" clause from the ScheduleWakeup note.
  - `mine/README.md` (~L60), `README.md` (~L78), `TERMS.md` (~L44): taskboard->board.

## 5. taskboard removal checklist
DELETE: `taskboard/` (dir, incl launch.sh+install.sh), `scripts/taskboard-resolve.sh`,
`mine/skills/taskboard/` (dir).
EDIT: plugin.json (entry), scripts/bootstrap.sh (function+call+vars), drill/SKILL.md,
assemble.md, oil/SKILL.md, default.md, coding-standards.md, mine/README.md, README.md,
TERMS.md, .gitignore (add `.board/`).
OIL-OWNED FOLLOW-UP (do NOT hand-edit — machine law: /oil owns /.machine/; queue an
/oil pass): `.machine/glossary.csv`, `.machine/specs/feature-factory/{SPEC,CONCEPT,CLOSE}.md`,
`.machine/plans/a1.md`.
Leave host-side taskboard binary/DB untouched (out of repo scope).

## 6. Glossary updates (queue for /oil; exact target content)
- Retire `process,taskboard` and rewrite `concept,board-per-cwd` (drop prefix/ULID specifics).
- Add `process,board` — "The local kanban addon: a single zero-dependency Node file
  (board/board.mjs, built mesh-style) serving an MCP verb surface over stdio plus a
  CDN-styled web UI and an SSE live-update channel on :3010, backed by one repo-scoped
  JSON state file under .board/. Projects->columns->cards->comments; the drill mirrors
  the /.machine/sessions/ ledger onto cards across lifecycle columns. The machine's sole
  board integration; design-templated on LocalBoards (MIT)."
- Rewrite `concept,board-per-cwd` — "One board project equals one working directory,
  resolved by name=basename(cwd) via project_resolve and persisted as a ULID projectId
  in .machine/board.json."

## 7. Risks / unknowns
- Realtime granularity: v1 pushes coarse `{rev}` over SSE, client refetches whole board
  — fine for a personal board. Keep the mutation->push hook in one place so upgrading to
  per-entity diffs is local.
- Concurrent writers: mesh-proven mkdir lock serializes web-UI writes and MCP writes to
  one state.json; drill + human dragging cannot corrupt. No DB/WAL needed.
- Native HTML5 DnD can be fiddly cross-browser: fallback is SortableJS CDN `<script>`.
- CDN offline: first paint needs network. If offline matters, inline a minimal
  stylesheet (page is classless-friendly). Note, do not pre-solve.

## 8. Verification checklist ("green")
- `node --check board/board.mjs` parses; `board --version` runs.
- `board mcp` answers initialize + tools/list over stdio; `mcp__board__project_resolve`
  etc. callable from the drill.
- `board serve` serves `/` (kanban renders, CDN style loads), `/api/board` returns
  grouped JSON, `/events` streams SSE `data:{rev}` on each change.
- Live reload: two tabs; card move in one appears in the other within ~1s (SSE refetch).
- Drag: moving a card between columns persists + survives reload.
- Comments: adding a comment on a card persists + shows.
- Projects: two repos (two cwds) get two projects; .machine/board.json holds the right id.
- Drill projection: resolve project -> 5 lifecycle columns -> ledger entry projected to a
  card -> moved across stages via card_move -> removed on merge via card_delete.
- taskboard gone: `grep -rn taskboard .` hits only the queued /.machine/ follow-up
  artifacts; taskboard/, scripts/taskboard-resolve.sh, mine/skills/taskboard/ deleted.
- Gate: `claude plugin validate . --strict`; plugin.json parses; hooks `node --check`;
  English-only/no-emoji standards. Run /gate. /personas clean (LocalBoards credited as
  design template; zero new deps).
- Addon-skip: with the web daemon down, MCP card ops still work (stdio independent of :3010).

## Files index
CREATE: `board/board.mjs`, `board/web/index.html`, `board/ATTRIBUTION.md`,
`mine/skills/board/SKILL.md`, `.machine/board.json` (runtime, gitignored).
MODIFY: `.claude-plugin/plugin.json`, `scripts/bootstrap.sh`,
`.claude/skills/drill/SKILL.md`, `.claude/skills/drill/references/assemble.md`,
`.claude/skills/oil/SKILL.md`, `.claude/agents/default.md`,
`.claude/rules/coding-standards.md`, `mine/README.md`, `README.md`, `TERMS.md`, `.gitignore`.
DELETE: `taskboard/`, `scripts/taskboard-resolve.sh`, `mine/skills/taskboard/`.
FOLLOW-UP (/oil, not hand-edited): `.machine/glossary.csv`,
`.machine/specs/feature-factory/*`, `.machine/plans/a1.md`.
