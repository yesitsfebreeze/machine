# Report: stale taskboard daemon squats :3010, board web never starts

- **When:** 2026-06-16
- **Component:** scripts/bootstrap.sh `start_board`; leftover `taskboard` daemon; board MCP registration
- **Severity:** high (board is mandatory for work tracking; the web view shows the wrong/old server)

## Symptom
- Port 3010 is held by the OLD `taskboard` Go daemon
  (`/home/feb/.local/bin/taskboard start --port 3010 --foreground`, pid 951007),
  which survived the b1 merge that removed the taskboard addon from the repo.
- `start_board` polls `curl http://localhost:3010/api/board` and treats ANY listener
  as "board already up", so the new `board/board.mjs serve` web daemon never starts.
- POST `/api/project_resolve` against :3010 returns taskboard's `404 page not found`
  (taskboard uses different routes), and `GET /api/board` returns taskboard's old
  todo/in_progress/done ticket model with stale cards from a prior session
  (project 01KV88RW..., cards a1/cc/dx/fi).

## Root cause
1. `start_board` liveness probe is not board-specific — it cannot tell the new
   `board.mjs` server from a stale taskboard squatting the same port.
2. Removing the taskboard addon in the b1 merge did not stop the running taskboard
   daemon (no `taskboard stop` in the merge/rollback path).
3. Board MCP tools are absent in sessions started before the 0.6.1 plugin gained the
   `board` mcpServers entry — they require a Claude Code restart to register. Several
   orphaned `board/board.mjs mcp` stdio processes from other sessions are also lingering.

## Suggested fix
- `start_board`: probe a board-specific endpoint/signature (e.g. verb surface or a
  `/healthz` returning the board's identity) before assuming "already up"; if the
  listener is not board, stop it (or fail loudly) and start `board.mjs serve`.
- Board rollback / taskboard removal must `taskboard stop` the running daemon.
- Document that adding the `board` mcpServers entry needs a harness restart to register.

## Workaround applied this session
- Killed the stale taskboard daemon, started `node board/board.mjs serve` on :3010,
  resolved the `machine` project + six lifecycle columns, and drove cards via the
  board HTTP verb surface (POST /api/<verb>) since the MCP server is not registered
  in this session.
