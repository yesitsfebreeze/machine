#!/usr/bin/env node
// machine — orchestrator control surface over Claude Code's native session layer + hub.
//
// One launched (orchestrator) Claude talks to hub; each board ticket gets its own
// detached, pinned headless Claude instance that registers itself in hub. The hub
// (singleton on :7777, reachable from every instance) is the roster of record.
//
//   machine ls                 merged roster: hub agents + native `claude agents`
//   machine dispatch <ticket> [task]   spawn a detached headless implementor for a ticket
//   machine tell <ticket> <msg>        send a live turn to a running implementor (stdin fifo)
//   machine jump <ticket>              attach a foreground session: claude --resume <id>
//   machine logs <ticket>              tail an implementor's output log
//   machine kill <ticket>              stop an implementor and release its claim
//
// Native facts this relies on (verified): --session-id pins a durable session;
// `claude --resume <id>` re-attaches; headless `-p --input-format stream-json
// --output-format stream-json --verbose` reads {"type":"user","message":{...}} lines
// and stays alive while stdin (an r+ fifo fd) is held open by the child itself.

import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { spawn, execFileSync } from "node:child_process";
import { randomUUID } from "node:crypto";

const HUB = process.env.MACHINE_HUB_URL || "http://localhost:7777";
const REPO = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const RUN = path.join(REPO, ".machine", "agents.json"); // ticket -> {uuid,pid,fifo,log,branch,startedAt}
const CLI_ID = "machine-cli";

// ---- small helpers ---------------------------------------------------------

// All hub verbs (board + mesh) are MCP tools; call them via JSON-RPC /mcp and
// unwrap the nested content text (itself a JSON string).
let mcpId = 0;
async function hub(verb, args) {
  const r = await fetch(`${HUB}/mcp`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ jsonrpc: "2.0", id: ++mcpId, method: "tools/call",
      params: { name: verb, arguments: args ?? {} } }),
  });
  const env = await r.json();
  if (env.error) throw new Error(`hub ${verb}: ${env.error.message || JSON.stringify(env.error)}`);
  const text = env.result?.content?.[0]?.text;
  if (text == null) return env.result ?? null;
  try { return JSON.parse(text); } catch { return text; }
}

function nativeAgents() {
  try {
    const out = execFileSync("claude", ["agents", "--json", "--cwd", REPO], { encoding: "utf8" });
    return JSON.parse(out);
  } catch {
    return [];
  }
}

function loadRun() {
  try { return JSON.parse(fs.readFileSync(RUN, "utf8")); } catch { return {}; }
}
function saveRun(db) {
  fs.mkdirSync(path.dirname(RUN), { recursive: true });
  fs.writeFileSync(RUN, JSON.stringify(db, null, 2));
}
function alive(pid) { try { process.kill(pid, 0); return true; } catch { return false; } }

// ---- commands --------------------------------------------------------------

async function cmdLs() {
  let agents = [];
  try {
    const r = await hub("roster", { agent_id: CLI_ID, include_stale: true });
    agents = r?.agents || [];
  } catch (e) {
    console.error(`(hub roster unavailable: ${e.message})`);
  }
  const native = nativeAgents();
  const db = loadRun();

  console.log("HUB ROSTER (agent_id · role · branch · liveness · session)");
  if (!agents.length) console.log("  (empty)");
  for (const a of agents) {
    const live = a.liveness === "live";
    console.log(`  ${live ? "●" : "○"} ${a.agent_id}\t${a.role || "-"}\t${a.branch || "-"}\t${a.liveness || "-"}\t${(db[a.agent_id]?.uuid || "-").slice(0, 8)}`);
  }

  console.log("\nNATIVE SESSIONS in this repo (kind · status · sessionId · name)");
  if (!native.length) console.log("  (none)");
  for (const s of native) {
    console.log(`  ${s.kind === "background" ? "▸" : "·"} ${s.kind}\t${s.status || s.state || "-"}\t${(s.sessionId || s.id || "").slice(0, 8)}\t${s.name || "-"}`);
  }

  console.log("\nMACHINE-MANAGED IMPLEMENTORS (.machine/agents.json)");
  const mine = Object.entries(db);
  if (!mine.length) console.log("  (none)");
  for (const [t, a] of mine) console.log(`  ${alive(a.pid) ? "●" : "✗"} ${t}\tpid ${a.pid}\t${a.uuid.slice(0, 8)}\t${a.log}`);
}

async function cmdDispatch(ticket, task) {
  if (!ticket) throw new Error("usage: machine dispatch <ticket> [task]");
  const db = loadRun();
  if (db[ticket] && alive(db[ticket].pid)) {
    console.error(`implementor for '${ticket}' already running (pid ${db[ticket].pid}). Use 'tell' or 'kill'.`);
    return;
  }
  const uuid = randomUUID();
  const branch = `t/${ticket}`;
  const dir = path.join(REPO, ".machine", "agents");
  fs.mkdirSync(dir, { recursive: true });
  const fifo = path.join(dir, `${ticket}.in`);
  const log = path.join(dir, `${ticket}.log`);
  try { fs.unlinkSync(fifo); } catch {}
  execFileSync("mkfifo", [fifo]);

  const role = `You are the implementor for board ticket "${ticket}".
First, register yourself in hub: call the mesh "register" tool with agent_id="${ticket}", role="implementor", branch="${branch}", prompt_ptr="ticket:${ticket}", and re-register before the TTL expires to stay live. Check your inbox with "inbox" for messages addressed to you. Report stage transitions with "post". Then work the ticket.`;

  const dry = process.argv.includes("--dry-run");
  const args = [
    "--name", ticket,
    "--session-id", uuid,
    "--append-system-prompt", role,
    "--permission-mode", "acceptEdits",
    ...(process.env.MACHINE_MODEL ? ["--model", process.env.MACHINE_MODEL] : []),
    "-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose",
  ];
  if (dry) {
    console.log(`DRY RUN — would launch in ${REPO}:`);
    console.log(`  claude ${args.map((a) => (/\s/.test(a) ? JSON.stringify(a) : a)).join(" ")}`);
    console.log(`  stdin <- fifo ${fifo}\n  stdout -> ${log}\n  first turn: ${task || "(none)"}`);
    return;
  }

  // r+ fd: the child holds a writer ref on its own stdin, so the fifo never EOFs
  // after this launcher exits. `tell` appends lines later.
  const stdinFd = fs.openSync(fifo, "r+");
  const logFd = fs.openSync(log, "a");
  const child = spawn("claude", args, {
    cwd: REPO, detached: true, stdio: [stdinFd, logFd, logFd],
  });
  child.unref();

  db[ticket] = { uuid, pid: child.pid, fifo, log, branch, startedAt: Date.now() };
  saveRun(db);

  if (task) fs.appendFileSync(fifo, JSON.stringify(userMsg(task)) + "\n");

  console.log(`dispatched '${ticket}'  pid=${child.pid}  session=${uuid}`);
  console.log(`  jump:  machine jump ${ticket}`);
  console.log(`  talk:  machine tell ${ticket} "..."`);
  console.log(`  logs:  machine logs ${ticket}`);

  // best-effort board reflection
  try {
    const bj = JSON.parse(fs.readFileSync(path.join(REPO, ".machine", "board.json"), "utf8"));
    const board = await hub("board_get", { projectId: bj.projectId });
    const cols = board?.columns || board?.project?.columns || [];
    const inProg = cols.find((c) => /in progress|implementing/i.test(c.name));
    if (inProg) {
      await hub("card_create", {
        columnId: inProg.id,
        title: `[implementing] ${ticket}`,
        body: `session: ${uuid}\nbranch: ${branch}\npid: ${child.pid}\ntask: ${task || ""}`,
        tags: ["machine"], assignee: ticket,
      });
      console.log(`  board: card created in "${inProg.name}"`);
    }
  } catch (e) { console.error(`  (board reflect skipped: ${e.message})`); }
}

function userMsg(text) {
  return { type: "user", message: { role: "user", content: text } };
}

async function cmdTell(ticket, ...rest) {
  const msg = rest.join(" ");
  if (!ticket || !msg) throw new Error('usage: machine tell <ticket> "message"');
  const db = loadRun();
  const a = db[ticket];
  if (!a) throw new Error(`no managed implementor '${ticket}' (try: machine dispatch ${ticket})`);
  if (!alive(a.pid)) console.error(`warning: pid ${a.pid} not alive; message may not be read`);
  fs.appendFileSync(a.fifo, JSON.stringify(userMsg(msg)) + "\n");
  console.log(`sent to '${ticket}'. Watch: machine logs ${ticket}`);
}

function cmdJump(ticket) {
  const db = loadRun();
  const a = db[ticket];
  if (!a) throw new Error(`no managed implementor '${ticket}'`);
  // foreground attach — replaces this process
  const r = spawn("claude", ["--resume", a.uuid], { stdio: "inherit", cwd: REPO });
  r.on("exit", (c) => process.exit(c ?? 0));
}

function cmdLogs(ticket) {
  const db = loadRun();
  const a = db[ticket];
  if (!a) throw new Error(`no managed implementor '${ticket}'`);
  spawn("tail", ["-n", "60", "-f", a.log], { stdio: "inherit" });
}

async function cmdKill(ticket) {
  const db = loadRun();
  const a = db[ticket];
  if (!a) throw new Error(`no managed implementor '${ticket}'`);
  try { process.kill(a.pid, "SIGTERM"); } catch {}
  try { await hub("post", { agent_id: CLI_ID, to: ticket, subject: "shutdown", body: "killed by machine cli" }); } catch {}
  try { fs.unlinkSync(a.fifo); } catch {}
  delete db[ticket];
  saveRun(db);
  console.log(`killed '${ticket}'`);
}

// ---- dispatch table --------------------------------------------------------

const [cmd, ...rest] = process.argv.slice(2);
const run = {
  ls: () => cmdLs(),
  dispatch: () => cmdDispatch(rest[0], rest.slice(1).filter((a) => a !== "--dry-run").join(" ") || ""),
  tell: () => cmdTell(rest[0], ...rest.slice(1)),
  jump: () => cmdJump(rest[0]),
  logs: () => cmdLogs(rest[0]),
  kill: () => cmdKill(rest[0]),
};
(async () => {
  try {
    if (!run[cmd]) {
      console.log("machine — orchestrator control surface\n");
      console.log("  machine ls");
      console.log("  machine dispatch <ticket> [task]   (+ --dry-run)");
      console.log("  machine tell <ticket> \"message\"");
      console.log("  machine jump <ticket>");
      console.log("  machine logs <ticket>");
      console.log("  machine kill <ticket>");
      process.exit(cmd ? 1 : 0);
    }
    await run[cmd]();
  } catch (e) {
    console.error(`error: ${e.message}`);
    process.exit(1);
  }
})();
