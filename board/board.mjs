#!/usr/bin/env node
// board — the machine's local kanban addon (b1-board).
//
// mesh's sibling for human-facing project state: where mesh owns live agent
// coordination and kern owns memory, board owns the at-a-glance kanban the drill
// projects its /.machine/sessions/ ledger onto. One zero-dependency Node file,
// built like mesh/mesh.mjs: a single JSON state file under a repo-scoped,
// gitignored `.board/` directory, an OS-atomic lock dir around every mutating op,
// and atomic rename on write.
//
// Two surfaces over one store:
//   board mcp    MCP server over stdio (card ops; the drill attaches here)
//   board serve  HTTP + SSE web UI on :3010 (a human view; live-reloads on change)
// Card operations work through MCP even when the web daemon is down.
//
// Data model (flattened from LocalBoards' MySQL tables; see board/ATTRIBUTION.md —
// design template only, no code copied): projects -> columns -> cards -> comments.
// Zero runtime dependencies — pure Node built-ins.

import {
  mkdirSync, readFileSync, writeFileSync, renameSync, rmdirSync, existsSync,
  unlinkSync, watch,
} from "node:fs";
import { randomBytes } from "node:crypto";
import { execFileSync } from "node:child_process";
import { createServer } from "node:http";
import { join, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// --- constants -------------------------------------------------------------

const PROTOCOL_VERSION = "2024-11-05";
const SERVER_VERSION = "0.1.0";
const DATA_DIR = ".board";
const STATE_FILE = "state.json";
const LOCK_DIR = ".lock";
const PID_FILE = "board.pid";
const DEFAULT_PORT = 3010;

const HERE = dirname(fileURLToPath(import.meta.url));
const WEB_INDEX = join(HERE, "web", "index.html");

// The domain verbs (single source of the tool surface, snake_case like mesh).
const VERBS = [
  "project_resolve", "project_list",
  "board_get",
  "column_create", "column_delete",
  "card_create", "card_update", "card_move", "card_delete",
  "comment_add", "comment_list",
];

// --- ULID: 48-bit time + 80-bit randomness, Crockford base32, monotonic -----

const B32 = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
let lastUlidMs = 0;
let lastRand = [];

function encodeTime(ms) {
  const out = new Array(10);
  for (let i = 9; i >= 0; i--) {
    out[i] = B32[ms % 32];
    ms = Math.floor(ms / 32);
  }
  return out.join("");
}

function freshRand() {
  const b = randomBytes(16);
  const r = new Array(16);
  for (let i = 0; i < 16; i++) r[i] = b[i] % 32;
  return r;
}

// Monotonic ULID: strictly increasing within a process even at sub-ms rate, so
// ids never tie and sort stably (used for card/column sort tie-breaks).
function ulid() {
  let t = Date.now();
  if (t <= lastUlidMs) {
    t = lastUlidMs;
    let i = 15;
    while (i >= 0) {
      if (lastRand[i] < 31) { lastRand[i]++; break; }
      lastRand[i] = 0; i--;
    }
  } else {
    lastRand = freshRand();
  }
  lastUlidMs = t;
  return encodeTime(t) + lastRand.map((x) => B32[x]).join("");
}

// --- time helper -----------------------------------------------------------

function nowIso() {
  return new Date().toISOString().replace(/\.\d{3}Z$/, "Z");
}

// --- the daemon ------------------------------------------------------------

export class Board {
  // `dir` is the `.board` data dir. `onMutate` (optional) fires after every
  // committed mutation with the new rev — the web layer hooks it to push SSE.
  constructor(dir, opts = {}) {
    this.dir = dir;
    this.statePath = join(dir, STATE_FILE);
    this.lockPath = join(dir, LOCK_DIR);
    this.onMutate = opts.onMutate || null;
    mkdirSync(dir, { recursive: true });
  }

  // --- storage: lock + atomic JSON ----------------------------------------

  #emptyState() {
    return { projects: {}, columns: {}, cards: {}, comments: {}, rev: 0 };
  }

  #load() {
    if (!existsSync(this.statePath)) return this.#emptyState();
    try {
      const s = JSON.parse(readFileSync(this.statePath, "utf8"));
      return { ...this.#emptyState(), ...s };
    } catch {
      return this.#emptyState();
    }
  }

  #save(state) {
    const tmp = `${this.statePath}.tmp.${process.pid}`;
    writeFileSync(tmp, JSON.stringify(state));
    renameSync(tmp, this.statePath); // atomic on POSIX and Windows same-volume
  }

  // OS-atomic mutex: mkdir of a lock dir succeeds for exactly one process.
  // Spin with backoff; steal a stale lock so a crashed holder can't wedge board.
  #lock() {
    const start = Date.now();
    for (;;) {
      try {
        mkdirSync(this.lockPath);
        return;
      } catch {
        if (Date.now() - start > 5000) {
          try { rmdirSync(this.lockPath); } catch { /* lost the race; loop */ }
        }
        const spinUntil = Date.now() + 2;
        while (Date.now() < spinUntil) { /* spin */ }
      }
    }
  }

  #unlock() {
    try { rmdirSync(this.lockPath); } catch { /* already released */ }
  }

  // Run `fn(state)` under the lock; persist whatever it returns via `mutated`.
  // On a committed mutation, bump rev and fire the live-push hook.
  #txn(fn) {
    this.#lock();
    let rev = null;
    try {
      const state = this.#load();
      const result = fn(state);
      if (result.mutated) {
        state.rev = (state.rev || 0) + 1;
        rev = state.rev;
        this.#save(state);
      }
      var value = result.value;
    } finally {
      this.#unlock();
    }
    if (rev != null && this.onMutate) this.onMutate(rev);
    return value;
  }

  // --- projects -----------------------------------------------------------

  // Get-or-create a project by name (board-per-cwd: name = basename(cwd)).
  project_resolve(req) {
    requireFields(req, ["name"]);
    return this.#txn((state) => {
      const existing = Object.values(state.projects).find((p) => p.name === req.name);
      if (existing) return { mutated: false, value: { project: existing } };
      const id = ulid();
      const project = { id, name: req.name, createdAt: nowIso() };
      state.projects[id] = project;
      return { mutated: true, value: { project } };
    });
  }

  project_list() {
    return this.#txn((state) => {
      const projects = Object.values(state.projects).sort((a, b) => cmp(a.createdAt, b.createdAt));
      return { mutated: false, value: { projects } };
    });
  }

  // --- board read ---------------------------------------------------------

  // The one read the UI and drill need: project + its columns (left-to-right by
  // sort) each carrying their cards (by sort) with comment counts.
  board_get(req) {
    requireFields(req, ["projectId"]);
    return this.#txn((state) => {
      const project = state.projects[req.projectId];
      if (!project) throw new BoardError(`unknown project '${req.projectId}'`);
      const commentCounts = {};
      for (const c of Object.values(state.comments)) {
        commentCounts[c.cardId] = (commentCounts[c.cardId] || 0) + 1;
      }
      const columns = Object.values(state.columns)
        .filter((col) => col.projectId === req.projectId)
        .sort(bySort)
        .map((col) => ({
          ...col,
          cards: Object.values(state.cards)
            .filter((k) => k.columnId === col.id)
            .sort(bySort)
            .map((k) => ({ ...k, commentCount: commentCounts[k.id] || 0 })),
        }));
      return { mutated: false, value: { project, columns } };
    });
  }

  // --- columns ------------------------------------------------------------

  column_create(req) {
    requireFields(req, ["projectId", "name"]);
    return this.#txn((state) => {
      if (!state.projects[req.projectId]) throw new BoardError(`unknown project '${req.projectId}'`);
      const id = ulid();
      const sort = nextSort(Object.values(state.columns).filter((c) => c.projectId === req.projectId));
      const column = { id, projectId: req.projectId, name: req.name, sort };
      state.columns[id] = column;
      return { mutated: true, value: { column } };
    });
  }

  // Delete a column and cascade to its cards and their comments.
  column_delete(req) {
    requireFields(req, ["id"]);
    return this.#txn((state) => {
      if (!state.columns[req.id]) return { mutated: false, value: { status: "unknown" } };
      delete state.columns[req.id];
      for (const card of Object.values(state.cards)) {
        if (card.columnId === req.id) {
          deleteCardCascade(state, card.id);
        }
      }
      return { mutated: true, value: { status: "deleted" } };
    });
  }

  // --- cards --------------------------------------------------------------

  card_create(req) {
    requireFields(req, ["columnId", "title"]);
    return this.#txn((state) => {
      if (!state.columns[req.columnId]) throw new BoardError(`unknown column '${req.columnId}'`);
      const id = ulid();
      const sort = nextSort(Object.values(state.cards).filter((k) => k.columnId === req.columnId));
      const card = {
        id, columnId: req.columnId, title: req.title,
        body: req.body ?? "", sort, createdAt: nowIso(),
      };
      state.cards[id] = card;
      return { mutated: true, value: { card } };
    });
  }

  card_update(req) {
    requireFields(req, ["id"]);
    return this.#txn((state) => {
      const card = state.cards[req.id];
      if (!card) throw new BoardError(`unknown card '${req.id}'`);
      if (req.title != null) card.title = req.title;
      if (req.body != null) card.body = req.body;
      return { mutated: true, value: { card } };
    });
  }

  // Move a card to a column at a 0-based index, renumbering the destination so
  // the requested slot is exact. Same-column reorder is supported too.
  card_move(req) {
    requireFields(req, ["id", "toColumnId"]);
    return this.#txn((state) => {
      const card = state.cards[req.id];
      if (!card) throw new BoardError(`unknown card '${req.id}'`);
      if (!state.columns[req.toColumnId]) throw new BoardError(`unknown column '${req.toColumnId}'`);
      card.columnId = req.toColumnId;
      const siblings = Object.values(state.cards)
        .filter((k) => k.columnId === req.toColumnId && k.id !== card.id)
        .sort(bySort);
      const idx = clamp(req.newIndex ?? siblings.length, 0, siblings.length);
      siblings.splice(idx, 0, card);
      siblings.forEach((k, i) => { k.sort = i; });
      return { mutated: true, value: { card } };
    });
  }

  card_delete(req) {
    requireFields(req, ["id"]);
    return this.#txn((state) => {
      if (!state.cards[req.id]) return { mutated: false, value: { status: "unknown" } };
      deleteCardCascade(state, req.id);
      return { mutated: true, value: { status: "deleted" } };
    });
  }

  // --- comments -----------------------------------------------------------

  comment_add(req) {
    requireFields(req, ["cardId", "author", "body"]);
    return this.#txn((state) => {
      if (!state.cards[req.cardId]) throw new BoardError(`unknown card '${req.cardId}'`);
      const id = ulid();
      const comment = { id, cardId: req.cardId, author: req.author, body: req.body, createdAt: nowIso() };
      state.comments[id] = comment;
      return { mutated: true, value: { comment } };
    });
  }

  comment_list(req) {
    requireFields(req, ["cardId"]);
    return this.#txn((state) => {
      const comments = Object.values(state.comments)
        .filter((c) => c.cardId === req.cardId)
        .sort((a, b) => cmp(a.createdAt, b.createdAt) || cmp(a.id, b.id));
      return { mutated: false, value: { comments } };
    });
  }
}

// --- pure helpers ----------------------------------------------------------

class BoardError extends Error {}

function requireFields(req, fields) {
  for (const f of fields) {
    if (req[f] == null || req[f] === "") throw new BoardError(`${fields.join(", ")} are required`);
  }
}

function cmp(a, b) {
  return a < b ? -1 : a > b ? 1 : 0;
}

function clamp(n, lo, hi) {
  return Math.min(hi, Math.max(lo, n));
}

// Sort by numeric `sort`, then by id for a stable total order.
function bySort(a, b) {
  return (a.sort - b.sort) || cmp(a.id, b.id);
}

// Next sort index = one past the current max (append to the end).
function nextSort(items) {
  return items.reduce((max, it) => Math.max(max, it.sort + 1), 0);
}

// Remove a card and every comment hanging off it.
function deleteCardCascade(state, cardId) {
  delete state.cards[cardId];
  for (const cid of Object.keys(state.comments)) {
    if (state.comments[cid].cardId === cardId) delete state.comments[cid];
  }
}

// --- MCP stdio loop --------------------------------------------------------

const TOOLS = {
  project_resolve: ["Get-or-create a board project by name (board-per-cwd).", {
    type: "object", required: ["name"],
    properties: { name: { type: "string" } },
  }],
  project_list: ["List all board projects.", {
    type: "object", properties: {},
  }],
  board_get: ["Read a project's columns and cards, grouped left-to-right with comment counts.", {
    type: "object", required: ["projectId"],
    properties: { projectId: { type: "string" } },
  }],
  column_create: ["Create a column (lifecycle lane) in a project.", {
    type: "object", required: ["projectId", "name"],
    properties: { projectId: { type: "string" }, name: { type: "string" } },
  }],
  column_delete: ["Delete a column and cascade to its cards and comments.", {
    type: "object", required: ["id"],
    properties: { id: { type: "string" } },
  }],
  card_create: ["Create a card in a column.", {
    type: "object", required: ["columnId", "title"],
    properties: { columnId: { type: "string" }, title: { type: "string" }, body: { type: "string" } },
  }],
  card_update: ["Update a card's title and/or body.", {
    type: "object", required: ["id"],
    properties: { id: { type: "string" }, title: { type: "string" }, body: { type: "string" } },
  }],
  card_move: ["Move a card to a column at a 0-based index (reorders the destination).", {
    type: "object", required: ["id", "toColumnId"],
    properties: { id: { type: "string" }, toColumnId: { type: "string" }, newIndex: { type: "integer" } },
  }],
  card_delete: ["Delete a card and its comments.", {
    type: "object", required: ["id"],
    properties: { id: { type: "string" } },
  }],
  comment_add: ["Add a comment to a card.", {
    type: "object", required: ["cardId", "author", "body"],
    properties: { cardId: { type: "string" }, author: { type: "string" }, body: { type: "string" } },
  }],
  comment_list: ["List a card's comments oldest-first.", {
    type: "object", required: ["cardId"],
    properties: { cardId: { type: "string" } },
  }],
};

function toolsList() {
  return { tools: VERBS.map((name) => ({ name, description: TOOLS[name][0], inputSchema: TOOLS[name][1] })) };
}

function dispatch(board, method, params) {
  switch (method) {
    case "initialize":
      return { protocolVersion: PROTOCOL_VERSION, capabilities: { tools: {} }, serverInfo: { name: "board", version: SERVER_VERSION } };
    case "notifications/initialized":
    case "initialized":
      return null;
    case "ping":
      return {};
    case "tools/list":
      return toolsList();
    case "tools/call": {
      if (!params) throw new BoardError("missing params");
      const name = params.name;
      if (!VERBS.includes(name)) throw new BoardError(`unknown verb '${name}'`);
      const result = board[name](params.arguments || {});
      return { content: [{ type: "text", text: JSON.stringify(result) }], structuredContent: result };
    }
    default:
      throw new BoardError(`unknown method '${method}'`);
  }
}

function handleLine(board, line, out) {
  let req;
  try {
    req = JSON.parse(line);
  } catch {
    out(JSON.stringify({ jsonrpc: "2.0", id: null, error: { code: -32700, message: "parse error" } }));
    return;
  }
  const id = req.id ?? null;
  const isNotification = req.id === undefined;
  try {
    const result = dispatch(board, req.method || "", req.params);
    if (result === null) {
      if (!isNotification) out(JSON.stringify({ jsonrpc: "2.0", id, result: {} }));
    } else {
      out(JSON.stringify({ jsonrpc: "2.0", id, result }));
    }
  } catch (e) {
    out(JSON.stringify({ jsonrpc: "2.0", id, error: { code: -32000, message: String(e.message || e) } }));
  }
}

function serveStdio(board) {
  let buf = "";
  process.stdin.setEncoding("utf8");
  process.stdin.on("data", (chunk) => {
    buf += chunk;
    let nl;
    while ((nl = buf.indexOf("\n")) >= 0) {
      const line = buf.slice(0, nl);
      buf = buf.slice(nl + 1);
      if (line.trim() === "") continue;
      handleLine(board, line, (s) => process.stdout.write(s + "\n"));
    }
  });
  process.stdin.on("end", () => process.exit(0));
}

// --- web server + SSE ------------------------------------------------------

// HTTP + SSE web view: GET / (UI), /healthz, /api/board (read), POST /api/<verb>
// (the full domain as thin JSON wrappers), and /events (live rev push).
function serveHttp(dataPath, port) {
  const clients = new Set(); // open SSE responses
  let lastRev = -1;
  const broadcast = (rev) => {
    if (rev == null || rev <= lastRev) return;
    lastRev = rev;
    const frame = `data:${JSON.stringify({ rev })}\n\n`;
    for (const res of clients) res.write(frame);
  };
  const board = new Board(dataPath, { onMutate: broadcast });

  // Cross-process live updates. The MCP server writes state.json from a SEPARATE
  // process, so its mutations never reach the in-process onMutate hook above —
  // without this watch, drill/MCP card changes would not live-refresh the page.
  // #save does writeFile(tmp)+rename, replacing the inode, so watch the data dir
  // (not the file) and re-broadcast whenever the persisted rev advances; rev
  // dedup in broadcast() collapses the duplicate from our own in-process writes.
  const statePath = join(dataPath, STATE_FILE);
  const readRev = () => {
    try { return JSON.parse(readFileSync(statePath, "utf8")).rev || 0; }
    catch { return null; }
  };
  lastRev = readRev() ?? -1;
  try {
    watch(dataPath, (_evt, file) => {
      if (file && file !== STATE_FILE) return;
      broadcast(readRev());
    });
  } catch { /* fs.watch unsupported here: in-process onMutate still works */ }

  let indexHtml = "";
  try { indexHtml = readFileSync(WEB_INDEX, "utf8"); } catch { /* served as 500 below */ }

  const server = createServer((req, res) => {
    const url = new URL(req.url, `http://localhost:${port}`);
    const send = (code, type, body) => {
      res.writeHead(code, { "content-type": type });
      res.end(body);
    };

    if (req.method === "GET" && url.pathname === "/") {
      if (!indexHtml) return send(500, "text/plain", "board: web/index.html missing");
      return send(200, "text/html; charset=utf-8", indexHtml);
    }

    // Board-specific liveness signature. Bootstrap probes THIS (not /api/board,
    // which a foreign daemon squatting the port may also answer 200 to) to tell a
    // real board.mjs server apart from a stale listener on the same port.
    if (req.method === "GET" && url.pathname === "/healthz") {
      return send(200, "application/json", JSON.stringify({ board: "machine-board", version: SERVER_VERSION }));
    }

    if (req.method === "GET" && url.pathname === "/api/board") {
      const projectId = url.searchParams.get("projectId");
      try {
        const value = projectId
          ? board.board_get({ projectId })
          : { projects: board.project_list().projects };
        return send(200, "application/json", JSON.stringify(value));
      } catch (e) {
        return send(400, "application/json", JSON.stringify({ error: String(e.message || e) }));
      }
    }

    // SSE live channel: every mutation pushes data:{"rev":N}; client refetches.
    if (req.method === "GET" && url.pathname === "/events") {
      res.writeHead(200, {
        "content-type": "text/event-stream",
        "cache-control": "no-cache",
        "connection": "keep-alive",
      });
      res.write(": connected\n\n");
      clients.add(res);
      req.on("close", () => clients.delete(res));
      return;
    }

    if (req.method === "POST" && url.pathname.startsWith("/api/")) {
      const verb = url.pathname.slice("/api/".length);
      if (!VERBS.includes(verb)) return send(404, "application/json", JSON.stringify({ error: `unknown verb '${verb}'` }));
      let raw = "";
      req.on("data", (c) => { raw += c; if (raw.length > 1e6) req.destroy(); });
      req.on("end", () => {
        let args = {};
        if (raw.trim()) {
          try { args = JSON.parse(raw); } catch { return send(400, "application/json", JSON.stringify({ error: "invalid JSON body" })); }
        }
        try {
          const value = board[verb](args);
          send(200, "application/json", JSON.stringify(value));
        } catch (e) {
          send(400, "application/json", JSON.stringify({ error: String(e.message || e) }));
        }
      });
      return;
    }

    send(404, "text/plain", "not found");
  });

  server.listen(port, () => {
    process.stdout.write(`board: serving http://localhost:${port}\n`);
  });
  return server;
}

// --- CLI -------------------------------------------------------------------

const USAGE = `board — the machine's local kanban addon (mesh's human-facing sibling)

Usage:
  board mcp        Run the MCP server over stdio (the drill attaches here)
  board serve      Serve the web UI + SSE on :${DEFAULT_PORT} (BOARD_PORT overrides)
  board stop       Stop a running web daemon (by pidfile)
  board --version  Show version

Data lives in a repo-scoped, gitignored .board/ directory at the repo root, shared
by every git worktree of the repo (a single JSON state file). Override with BOARD_DIR.
Zero runtime dependencies — needs only Node.`;

// Resolve the board store. Repo-scoped, not cwd-scoped: every git worktree of the
// same repository shares one store (parity with mesh). Resolution order:
//   1. BOARD_DIR env override.
//   2. The repo root that owns this cwd (parent of git's common dir).
//   3. Fall back to cwd when not inside a git repository.
function dataDir() {
  if (process.env.BOARD_DIR) return resolve(process.env.BOARD_DIR);
  try {
    const common = execFileSync("git", ["rev-parse", "--git-common-dir"], {
      cwd: process.cwd(),
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    if (common) {
      return join(dirname(resolve(process.cwd(), common)), DATA_DIR);
    }
  } catch {
    // not a git repo, or git unavailable — fall through to cwd
  }
  return join(process.cwd(), DATA_DIR);
}

function port() {
  const p = parseInt(process.env.BOARD_PORT || "", 10);
  return Number.isInteger(p) && p > 0 ? p : DEFAULT_PORT;
}

function pidPath(dir) {
  return join(dir, PID_FILE);
}

function main(argv) {
  const cmd = argv[0] || "";
  const dir = dataDir();
  switch (cmd) {
    case "mcp":
      serveStdio(new Board(dir));
      break;
    case "serve": {
      const pf = pidPath(dir);
      mkdirSync(dir, { recursive: true });
      writeFileSync(pf, String(process.pid));
      const server = serveHttp(dir, port());
      const shutdown = () => {
        try { unlinkSync(pf); } catch { /* gone */ }
        server.close(() => process.exit(0));
        setTimeout(() => process.exit(0), 500).unref();
      };
      process.on("SIGINT", shutdown);
      process.on("SIGTERM", shutdown);
      break;
    }
    case "stop": {
      const pf = pidPath(dir);
      if (!existsSync(pf)) { process.stdout.write("board: no running daemon (no pidfile)\n"); break; }
      const pid = parseInt(readFileSync(pf, "utf8").trim(), 10);
      try {
        process.kill(pid, "SIGTERM");
        process.stdout.write(`board: stopped daemon (pid ${pid})\n`);
      } catch {
        process.stdout.write(`board: daemon not running (stale pidfile, pid ${pid})\n`);
      }
      try { unlinkSync(pf); } catch { /* gone */ }
      break;
    }
    case "--version":
    case "-V":
      process.stdout.write(`board ${SERVER_VERSION}\n`);
      break;
    case "":
    case "--help":
    case "-h":
    case "help":
      process.stdout.write(USAGE + "\n");
      break;
    default:
      process.stderr.write(`board: unknown command '${cmd}'\n\n${USAGE}\n`);
      process.exit(1);
  }
}

// Run as CLI only when invoked directly (not when imported by tests).
if (import.meta.url === `file://${process.argv[1]}`) {
  main(process.argv.slice(2));
}
