#!/usr/bin/env node
// mesh — fleet inter-agent communication daemon (SPEC-COMM-001).
//
// kern's coordination sibling: kern owns *memory* (why decisions were made),
// mesh owns *live coordination* (who is here, who holds what, who said what to
// whom). Three categories of dynamic state — roster, claims, messages — exposed
// as eight MCP verbs over stdio.
//
// This is a zero-dependency rewrite of the original Rust daemon. It needs nothing
// but a Node runtime (already required by the machine for context-mode). State is
// a single JSON file under a per-cwd, gitignored `.mesh/` directory; cross-process
// atomicity (the CAS the claim primitive needs) comes from an OS-atomic lock
// directory held around every mutating op, plus atomic rename on write.

import { mkdirSync, readFileSync, writeFileSync, renameSync, rmdirSync, existsSync } from "node:fs";
import { randomBytes } from "node:crypto";
import { join } from "node:path";

// --- constants (parity with the Rust daemon) -------------------------------

const PROTOCOL_VERSION = "2024-11-05";
const SERVER_VERSION = "0.2.0";
const DATA_DIR = ".mesh";
const STATE_FILE = "state.json";
const LOCK_DIR = ".lock";

const DEFAULT_TTL_SECONDS = 60; // roster heartbeat window
const STALE_GRACE_SECONDS = 30; // grace after expiry before `dead`
const DEFAULT_LEASE_SECONDS = 120; // claim hold window
const ZERO_CURSOR = "00000000000000000000000000"; // below every ULID

// The eight verbs (single source of the tool surface).
const VERBS = ["register", "roster", "claim", "release", "claims", "post", "inbox", "read"];

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
// the cursor total-order never ties.
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

// --- time helpers ----------------------------------------------------------

// RFC3339 UTC at second precision, e.g. 2026-06-14T12:00:00Z (matches Rust).
function isoFromUnix(unixSeconds) {
  return new Date(unixSeconds * 1000).toISOString().replace(/\.\d{3}Z$/, "Z");
}

// --- the daemon ------------------------------------------------------------

export class Mesh {
  // `dir` is the `.mesh` data dir. `now` returns Unix seconds (injectable for tests).
  constructor(dir, opts = {}) {
    this.dir = dir;
    this.statePath = join(dir, STATE_FILE);
    this.lockPath = join(dir, LOCK_DIR);
    this.now = opts.now || (() => Math.floor(Date.now() / 1000));
    mkdirSync(dir, { recursive: true });
  }

  // --- storage: lock + atomic JSON ----------------------------------------

  #emptyState() {
    return { roster: {}, claims: {}, messages: {}, log: [], cursors: {}, events: [], fence_floor: {} };
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
  // Spin with backoff; steal a stale lock so a crashed holder can't wedge mesh.
  #lock() {
    const start = Date.now();
    for (;;) {
      try {
        mkdirSync(this.lockPath);
        return;
      } catch {
        const waited = Date.now() - start;
        if (waited > 5000) {
          // Assume the holder died mid-op; reclaim and retry once.
          try { rmdirSync(this.lockPath); } catch { /* lost the race; loop */ }
        }
        // Busy-wait briefly (sub-ms ops; contention windows are tiny).
        const spinUntil = Date.now() + 2;
        while (Date.now() < spinUntil) { /* spin */ }
      }
    }
  }

  #unlock() {
    try { rmdirSync(this.lockPath); } catch { /* already released */ }
  }

  // Run `fn(state)` under the lock; persist whatever it returns via `mutated`.
  #txn(fn) {
    this.#lock();
    try {
      const state = this.#load();
      const result = fn(state);
      if (result.mutated) this.#save(state);
      return result.value;
    } finally {
      this.#unlock();
    }
  }

  // --- awareness: register / roster ---------------------------------------

  register(req) {
    requireFields(req, ["agent_id", "branch", "prompt_ptr"]);
    const now = this.now();
    const ttl = Math.max(1, req.ttl_seconds ?? DEFAULT_TTL_SECONDS);
    return this.#txn((state) => {
      const prev = state.roster[req.agent_id];
      let registered_at = now, epoch = 1;
      if (prev) {
        registered_at = prev.registered_at;
        const wasDead = livenessAt(prev.expires_at, now) === "dead";
        epoch = wasDead ? prev.epoch + 1 : prev.epoch;
      }
      const rec = {
        agent_id: req.agent_id,
        branch: req.branch,
        prompt_ptr: req.prompt_ptr,
        role: req.role ?? null,
        registered_at,
        last_seen: now,
        expires_at: now + ttl,
        epoch,
      };
      state.roster[req.agent_id] = rec;
      return {
        mutated: true,
        value: {
          agent_id: rec.agent_id,
          registered_at: isoFromUnix(rec.registered_at),
          expires_at: isoFromUnix(rec.expires_at),
          epoch: rec.epoch,
        },
      };
    });
  }

  roster(req) {
    requireFields(req, ["agent_id"]);
    const now = this.now();
    return this.#txn((state) => {
      const changed = sweep(state, now);
      const held = heldClaimsIndex(state, now);
      const agents = [];
      for (const rec of Object.values(state.roster)) {
        const liveness = livenessAt(rec.expires_at, now);
        if (liveness === "dead" && !req.include_stale) continue;
        const entry = {
          agent_id: rec.agent_id,
          branch: rec.branch,
          prompt_ptr: rec.prompt_ptr,
          liveness,
          last_seen: isoFromUnix(rec.last_seen),
          expires_at: isoFromUnix(rec.expires_at),
          held_claims: (held[rec.agent_id] || []).slice().sort(),
        };
        if (rec.role != null) entry.role = rec.role;
        agents.push(entry);
      }
      agents.sort((a, b) => cmp(a.agent_id, b.agent_id));
      return { mutated: changed, value: { agents } };
    });
  }

  // --- claims: claim / release / claims -----------------------------------

  claim(req) {
    if (!req.agent_id || !req.resource) {
      throw new MeshError("agent_id and resource are required");
    }
    const now = this.now();
    const mode = req.mode || "exclusive";
    const lease = Math.max(1, req.lease_seconds ?? DEFAULT_LEASE_SECONDS);
    const wait = req.wait || "no_wait";
    return this.#txn((state) => {
      let rec = state.claims[req.resource] || {
        resource: req.resource,
        mode,
        holders: [],
        queue: [],
        fence: state.fence_floor[req.resource] || 0,
      };
      reapRecord(state, rec, now);

      // Idempotent renewal by the current holder.
      const hpos = rec.holders.findIndex((h) => h.agent_id === req.agent_id);
      if (hpos >= 0) {
        const holder = rec.holders[hpos];
        holder.lease_expires_at = now + lease;
        if (req.note != null) holder.note = req.note;
        state.claims[req.resource] = rec;
        return {
          mutated: true,
          value: {
            status: "granted",
            resource: req.resource,
            claim_id: holder.claim_id,
            holder: req.agent_id,
            lease_expires_at: isoFromUnix(holder.lease_expires_at),
            fence: rec.fence,
          },
        };
      }

      if (grantable(rec, mode)) {
        const claim_id = ulid();
        rec.mode = mode;
        rec.fence += 1;
        rec.holders.push({ agent_id: req.agent_id, claim_id, lease_expires_at: now + lease, note: req.note ?? null });
        state.claims[req.resource] = rec;
        state.events.push({ resource: req.resource, event: "grant", agent_id: req.agent_id, fence: rec.fence, at: isoFromUnix(now) });
        return {
          mutated: true,
          value: {
            status: "granted",
            resource: req.resource,
            claim_id,
            holder: req.agent_id,
            lease_expires_at: isoFromUnix(now + lease),
            fence: rec.fence,
          },
        };
      }

      // Held by someone else: queue or deny.
      const currentHolder = rec.holders[0]?.agent_id;
      if (wait === "queue") {
        const claim_id = ulid();
        rec.queue.push({ agent_id: req.agent_id, claim_id, mode, lease_seconds: lease, note: req.note ?? null });
        state.claims[req.resource] = rec;
        const value = {
          status: "queued",
          resource: req.resource,
          claim_id,
          queue_position: rec.queue.length,
          fence: rec.fence,
        };
        if (currentHolder != null) value.holder = currentHolder;
        return { mutated: true, value };
      }
      // no_wait: persist any reaping, grant nothing.
      state.claims[req.resource] = rec;
      const value = { status: "denied", resource: req.resource, claim_id: "", fence: rec.fence };
      if (currentHolder != null) value.holder = currentHolder;
      return { mutated: true, value };
    });
  }

  release(req) {
    requireFields(req, ["agent_id", "claim_id", "resource"]);
    const now = this.now();
    return this.#txn((state) => {
      const rec = state.claims[req.resource];
      if (!rec) return { mutated: false, value: { status: "unknown" } };

      const hi = rec.holders.findIndex((h) => h.agent_id === req.agent_id && h.claim_id === req.claim_id);
      const qi = rec.queue.findIndex((t) => t.agent_id === req.agent_id && t.claim_id === req.claim_id);

      if (hi < 0 && qi < 0) {
        const known =
          rec.holders.some((h) => h.claim_id === req.claim_id) ||
          rec.queue.some((t) => t.claim_id === req.claim_id);
        return { mutated: false, value: { status: known ? "not_holder" : "unknown" } };
      }

      if (qi >= 0) {
        rec.queue.splice(qi, 1); // cancel queued ticket, no promotion
        state.claims[req.resource] = rec;
        return { mutated: true, value: { status: "released", fence: rec.fence } };
      }

      rec.holders.splice(hi, 1);
      state.events.push({ resource: req.resource, event: "release", agent_id: req.agent_id, fence: rec.fence, at: isoFromUnix(now) });
      const promoted = promoteQueue(rec, now);
      if (promoted) state.events.push({ resource: req.resource, event: "promote", agent_id: promoted, fence: rec.fence, at: isoFromUnix(now) });
      const fence = rec.fence;
      persistOrDelete(state, req.resource, rec);
      const value = { status: "released", fence };
      if (promoted) value.next_holder = promoted;
      return { mutated: true, value };
    });
  }

  claims(req) {
    requireFields(req, ["agent_id"]);
    const now = this.now();
    return this.#txn((state) => {
      const changed = sweep(state, now);
      const out = [];
      for (const rec of Object.values(state.claims)) {
        if (rec.holders.length === 0) continue;
        if (req.resource && rec.resource !== req.resource) continue;
        out.push(claimView(rec));
      }
      out.sort((a, b) => cmp(a.resource, b.resource));
      return { mutated: changed, value: { claims: out } };
    });
  }

  // --- messaging: post / inbox / read -------------------------------------

  post(req) {
    if (!req.agent_id || !req.to || !req.body) {
      throw new MeshError("agent_id, to, and body are required");
    }
    const now = this.now();
    const message_id = ulid();
    const expires_unix = req.ttl_seconds != null ? now + Math.max(1, req.ttl_seconds) : null;
    return this.#txn((state) => {
      state.messages[message_id] = {
        message_id,
        from: req.agent_id,
        to: req.to,
        subject: req.subject ?? null,
        body: req.body,
        reply_to: req.reply_to ?? null,
        posted_at: now,
        expires_at: expires_unix,
      };
      state.log.push({ message_id, sender: req.agent_id, recipient: req.to, posted_at: isoFromUnix(now), expires_unix });
      const fanout = req.to === "*" ? Object.keys(state.roster).length : req.to.startsWith("topic:") ? 0 : 1;
      return { mutated: true, value: { message_id, posted_at: isoFromUnix(now), fanout } };
    });
  }

  inbox(req) {
    requireFields(req, ["agent_id"]);
    const now = this.now();
    const limit = clamp(req.limit ?? 100, 1, 1000);
    const topics = req.topics || [];
    return this.#txn((state) => {
      const liveCursor = state.cursors[req.agent_id] || ZERO_CURSOR;
      const cursor = req.since || liveCursor;
      const pending = pendingAfter(state, req.agent_id, cursor, topics, now);
      const messages = [];
      for (const id of pending.slice(0, limit)) {
        const m = state.messages[id];
        if (m) messages.push(toInboxMessage(m));
      }
      return {
        mutated: false,
        value: { messages, cursor: liveCursor, unread: Math.max(0, pending.length - messages.length) },
      };
    });
  }

  read(req) {
    requireFields(req, ["agent_id", "up_to"]);
    const now = this.now();
    return this.#txn((state) => {
      const current = state.cursors[req.agent_id] || ZERO_CURSOR;
      const newCursor = req.up_to > current ? req.up_to : current;
      state.cursors[req.agent_id] = newCursor;
      const remaining = pendingAfter(state, req.agent_id, newCursor, [], now).length;
      return { mutated: true, value: { cursor: newCursor, remaining } };
    });
  }

  // --- maintenance: gc ----------------------------------------------------

  // Drop TTL-expired messages and sweep dead claims. Returns reclaimed count.
  gc() {
    const now = this.now();
    return this.#txn((state) => {
      sweep(state, now);
      let reclaimed = 0;
      state.log = state.log.filter((row) => {
        const expired = row.expires_unix != null && row.expires_unix <= now;
        if (expired) {
          delete state.messages[row.message_id];
          reclaimed++;
        }
        return !expired;
      });
      return { mutated: true, value: reclaimed };
    });
  }
}

// --- pure helpers ----------------------------------------------------------

class MeshError extends Error {}

function requireFields(req, fields) {
  for (const f of fields) {
    if (req[f] == null || req[f] === "") throw new MeshError(`${fields.join(", ")} are required`);
  }
}

function cmp(a, b) {
  return a < b ? -1 : a > b ? 1 : 0;
}

function clamp(n, lo, hi) {
  return Math.min(hi, Math.max(lo, n));
}

function livenessAt(expiresAt, now) {
  if (now <= expiresAt) return "alive";
  if (now <= expiresAt + STALE_GRACE_SECONDS) return "stale";
  return "dead";
}

function agentIsDead(state, agentId, now) {
  const rec = state.roster[agentId];
  return rec ? livenessAt(rec.expires_at, now) === "dead" : false;
}

function grantable(rec, mode) {
  if (rec.holders.length === 0) return true;
  return rec.mode === "shared" && mode === "shared";
}

function promoteQueue(rec, now) {
  if (rec.holders.length > 0 || rec.queue.length === 0) return null;
  const ticket = rec.queue.shift();
  rec.mode = ticket.mode;
  rec.fence += 1;
  const first = ticket.agent_id;
  rec.holders.push(holderFromTicket(ticket, now));
  if (rec.mode === "shared") {
    while (rec.queue[0] && rec.queue[0].mode === "shared") {
      rec.holders.push(holderFromTicket(rec.queue.shift(), now));
    }
  }
  return first;
}

function holderFromTicket(ticket, now) {
  return { agent_id: ticket.agent_id, claim_id: ticket.claim_id, lease_expires_at: now + ticket.lease_seconds, note: ticket.note ?? null };
}

// Reap one record in place: drop expired/dead holders, dead-agent queue
// tickets, then promote a waiter if the resource went free. Returns whether it
// changed.
function reapRecord(state, rec, now) {
  const beforeH = rec.holders.length;
  const beforeQ = rec.queue.length;
  rec.holders = rec.holders.filter((h) => h.lease_expires_at > now && !agentIsDead(state, h.agent_id, now));
  rec.queue = rec.queue.filter((t) => !agentIsDead(state, t.agent_id, now));
  let changed = rec.holders.length !== beforeH || rec.queue.length !== beforeQ;
  if (rec.holders.length === 0 && rec.queue.length > 0 && promoteQueue(rec, now)) changed = true;
  return changed;
}

// Sweep every claim record; delete fully idle ones (stamping the fence floor).
function sweep(state, now) {
  let changed = false;
  for (const resource of Object.keys(state.claims)) {
    const rec = state.claims[resource];
    if (reapRecord(state, rec, now)) {
      changed = true;
      persistOrDelete(state, resource, rec);
    }
  }
  return changed;
}

function persistOrDelete(state, resource, rec) {
  if (rec.holders.length === 0 && rec.queue.length === 0) {
    state.fence_floor[resource] = rec.fence; // resume monotonically on re-acquire
    delete state.claims[resource];
  } else {
    state.claims[resource] = rec;
  }
}

function heldClaimsIndex(state, now) {
  const index = {};
  for (const rec of Object.values(state.claims)) {
    for (const h of rec.holders) {
      if (h.lease_expires_at > now) (index[h.agent_id] ||= []).push(rec.resource);
    }
  }
  return index;
}

function claimView(rec) {
  const holders = rec.holders.map((h) => h.agent_id);
  const view = {
    resource: rec.resource,
    mode: rec.mode,
    holder: holders,
    claim_id: rec.holders[0]?.claim_id ?? "",
    fence: rec.fence,
    lease_expires_at: isoFromUnix(Math.min(...rec.holders.map((h) => h.lease_expires_at), Infinity) === Infinity ? 0 : Math.min(...rec.holders.map((h) => h.lease_expires_at))),
    queue: rec.queue.map((t, i) => ({ agent_id: t.agent_id, claim_id: t.claim_id, position: i + 1 })),
  };
  const note = rec.holders[0]?.note;
  if (note != null) view.note = note;
  return view;
}

function addressedTo(recipient, agentId, topics) {
  if (recipient === agentId) return true;
  if (recipient === "*") return true;
  if (recipient.startsWith("topic:")) {
    const name = recipient.slice("topic:".length);
    return topics.some((t) => (t.startsWith("topic:") ? t.slice("topic:".length) : t) === name);
  }
  return false;
}

// Ordered message_ids addressed to `agentId`, id strictly above `cursor`, unexpired.
function pendingAfter(state, agentId, cursor, topics, now) {
  return state.log
    .filter((row) => row.message_id > cursor)
    .filter((row) => !(row.expires_unix != null && row.expires_unix <= now))
    .filter((row) => addressedTo(row.recipient, agentId, topics))
    .sort((a, b) => cmp(a.message_id, b.message_id))
    .map((row) => row.message_id);
}

function toInboxMessage(rec) {
  const m = { message_id: rec.message_id, from: rec.from, to: rec.to, body: rec.body, posted_at: isoFromUnix(rec.posted_at) };
  if (rec.subject != null) m.subject = rec.subject;
  if (rec.reply_to != null) m.reply_to = rec.reply_to;
  return m;
}

// --- MCP stdio loop --------------------------------------------------------

const TOOLS = {
  register: ["Announce presence and refresh liveness (heartbeat).", {
    type: "object", required: ["agent_id", "branch", "prompt_ptr"],
    properties: { agent_id: { type: "string" }, branch: { type: "string" }, prompt_ptr: { type: "string" }, role: { type: "string" }, ttl_seconds: { type: "integer" } },
  }],
  roster: ["List known agents and their liveness.", {
    type: "object", required: ["agent_id"],
    properties: { agent_id: { type: "string" }, include_stale: { type: "boolean" } },
  }],
  claim: ["Atomically acquire (or queue for) a resource lock.", {
    type: "object", required: ["agent_id", "resource"],
    properties: { agent_id: { type: "string" }, resource: { type: "string" }, mode: { type: "string", enum: ["exclusive", "shared"] }, lease_seconds: { type: "integer" }, wait: { type: "string", enum: ["no_wait", "queue"] }, note: { type: "string" } },
  }],
  release: ["Relinquish a held claim or cancel a queued ticket.", {
    type: "object", required: ["agent_id", "claim_id", "resource"],
    properties: { agent_id: { type: "string" }, claim_id: { type: "string" }, resource: { type: "string" } },
  }],
  claims: ["Inspect current locks and queues.", {
    type: "object", required: ["agent_id"],
    properties: { agent_id: { type: "string" }, resource: { type: "string" } },
  }],
  post: ["Send a durable message (agent_id, * broadcast, or topic:<name>).", {
    type: "object", required: ["agent_id", "to", "body"],
    properties: { agent_id: { type: "string" }, to: { type: "string" }, subject: { type: "string" }, body: { type: "string" }, reply_to: { type: "string" }, ttl_seconds: { type: "integer" } },
  }],
  inbox: ["Peek pending messages without advancing the cursor.", {
    type: "object", required: ["agent_id"],
    properties: { agent_id: { type: "string" }, since: { type: "string" }, topics: { type: "array", items: { type: "string" } }, limit: { type: "integer" } },
  }],
  read: ["Advance the read cursor (acknowledge consumption).", {
    type: "object", required: ["agent_id", "up_to"],
    properties: { agent_id: { type: "string" }, up_to: { type: "string" } },
  }],
};

function toolsList() {
  return { tools: VERBS.map((name) => ({ name, description: TOOLS[name][0], inputSchema: TOOLS[name][1] })) };
}

function dispatch(mesh, method, params) {
  switch (method) {
    case "initialize":
      return { protocolVersion: PROTOCOL_VERSION, capabilities: { tools: {} }, serverInfo: { name: "mesh", version: SERVER_VERSION } };
    case "notifications/initialized":
    case "initialized":
      return null;
    case "ping":
      return {};
    case "tools/list":
      return toolsList();
    case "tools/call": {
      if (!params) throw new MeshError("missing params");
      const name = params.name;
      if (!VERBS.includes(name)) throw new MeshError(`unknown verb '${name}'`);
      const result = mesh[name](params.arguments || {});
      return { content: [{ type: "text", text: JSON.stringify(result) }], structuredContent: result };
    }
    default:
      throw new MeshError(`unknown method '${method}'`);
  }
}

function handleLine(mesh, line, out) {
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
    const result = dispatch(mesh, req.method || "", req.params);
    if (result === null) {
      if (!isNotification) out(JSON.stringify({ jsonrpc: "2.0", id, result: {} }));
    } else {
      out(JSON.stringify({ jsonrpc: "2.0", id, result }));
    }
  } catch (e) {
    out(JSON.stringify({ jsonrpc: "2.0", id, error: { code: -32000, message: String(e.message || e) } }));
  }
}

function serveStdio(mesh) {
  let buf = "";
  process.stdin.setEncoding("utf8");
  process.stdin.on("data", (chunk) => {
    buf += chunk;
    let nl;
    while ((nl = buf.indexOf("\n")) >= 0) {
      const line = buf.slice(0, nl);
      buf = buf.slice(nl + 1);
      if (line.trim() === "") continue;
      handleLine(mesh, line, (s) => process.stdout.write(s + "\n"));
    }
  });
  process.stdin.on("end", () => process.exit(0));
}

// --- CLI -------------------------------------------------------------------

const USAGE = `mesh — fleet inter-agent communication daemon (kern's coordination sibling)

Usage:
  mesh mcp        Run the MCP server over stdio (the fleet attaches here)
  mesh gc         Reclaim TTL-expired messages and sweep dead claims
  mesh --version  Show version

Data lives in a per-cwd, gitignored .mesh/ directory (a single JSON state file).
Zero runtime dependencies — needs only Node.`;

function dataDir() {
  return join(process.cwd(), DATA_DIR);
}

function main(argv) {
  const cmd = argv[0] || "";
  switch (cmd) {
    case "mcp":
      serveStdio(new Mesh(dataDir()));
      break;
    case "gc":
    case "compact": {
      const n = new Mesh(dataDir()).gc();
      process.stdout.write(`mesh: reclaimed ${n} expired message(s); dead claims swept\n`);
      break;
    }
    case "--version":
    case "-V":
      process.stdout.write(`mesh ${SERVER_VERSION}\n`);
      break;
    case "":
    case "--help":
    case "-h":
    case "help":
      process.stdout.write(USAGE + "\n");
      break;
    default:
      process.stderr.write(`mesh: unknown command '${cmd}'\n\n${USAGE}\n`);
      process.exit(1);
  }
}

// Run as CLI only when invoked directly (not when imported by tests).
if (import.meta.url === `file://${process.argv[1]}`) {
  main(process.argv.slice(2));
}
