#!/usr/bin/env node
// Acceptance suite for the mjs mesh daemon. Zero deps; run: `node mesh/test.mjs`.
// Mirrors the highest-risk SPEC criteria the Rust suite covered.

import { Mesh } from "./mesh.mjs";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

let passed = 0, failed = 0;
function ok(cond, name) {
  if (cond) { passed++; }
  else { failed++; console.error(`  FAIL: ${name}`); }
}
function eq(a, b, name) {
  ok(JSON.stringify(a) === JSON.stringify(b), `${name} (got ${JSON.stringify(a)}, want ${JSON.stringify(b)})`);
}

// Controllable clock so lease/liveness tests need no sleeping.
function fixture() {
  const dir = mkdtempSync(join(tmpdir(), "mesh-test-"));
  let t = 1_000_000;
  const m = new Mesh(join(dir, ".mesh"), { now: () => t });
  return { m, dir, advance: (s) => { t += s; }, at: () => t };
}

function test(name, fn) {
  const f = fixture();
  try { fn(f); }
  catch (e) { failed++; console.error(`  THREW in ${name}: ${e.stack || e}`); }
  finally { rmSync(f.dir, { recursive: true, force: true }); }
}

// --- awareness -------------------------------------------------------------

test("register + roster + liveness state machine", ({ m, advance }) => {
  const r = m.register({ agent_id: "a", branch: "agent/a", prompt_ptr: "p", ttl_seconds: 60 });
  eq(r.epoch, 1, "first register epoch=1");
  eq(m.roster({ agent_id: "a" }).agents[0].liveness, "alive", "alive within ttl");
  advance(75); // 60 ttl + into grace
  eq(m.roster({ agent_id: "a", include_stale: true }).agents[0].liveness, "stale", "stale in grace");
  advance(30); // past grace -> dead
  eq(m.roster({ agent_id: "a" }).agents.length, 0, "dead hidden by default");
  eq(m.roster({ agent_id: "a", include_stale: true }).agents[0].liveness, "dead", "dead with include_stale");
  const r2 = m.register({ agent_id: "a", branch: "agent/a", prompt_ptr: "p" });
  eq(r2.epoch, 2, "re-register after death bumps epoch");
});

// --- claims ----------------------------------------------------------------

test("exclusive grant, deny, queue, promote, fence monotonicity", ({ m }) => {
  const g = m.claim({ agent_id: "a", resource: "R" });
  eq(g.status, "granted", "first exclusive granted");
  eq(g.fence, 1, "fence starts at 1");
  const d = m.claim({ agent_id: "b", resource: "R" });
  eq(d.status, "denied", "second denied (no_wait)");
  eq(d.holder, "a", "denied reports holder");
  const q = m.claim({ agent_id: "c", resource: "R", wait: "queue" });
  eq(q.status, "queued", "third queued");
  eq(q.queue_position, 1, "queue position 1");
  const rel = m.release({ agent_id: "a", claim_id: g.claim_id, resource: "R" });
  eq(rel.status, "released", "release ok");
  eq(rel.next_holder, "c", "queue promoted on release");
  const view = m.claims({ agent_id: "x" }).claims[0];
  eq(view.holder, ["c"], "c now holds R");
  ok(view.fence === 2, "fence bumped to 2 on promotion");
});

test("idempotent renewal by holder keeps claim_id and fence", ({ m, advance }) => {
  const g = m.claim({ agent_id: "a", resource: "R", lease_seconds: 100 });
  advance(50);
  const g2 = m.claim({ agent_id: "a", resource: "R", lease_seconds: 100 });
  eq(g2.claim_id, g.claim_id, "renewal keeps claim_id");
  eq(g2.fence, g.fence, "renewal does not bump fence");
});

test("shared co-holders, exclusive blocked", ({ m }) => {
  eq(m.claim({ agent_id: "a", resource: "S", mode: "shared" }).status, "granted", "shared 1 granted");
  eq(m.claim({ agent_id: "b", resource: "S", mode: "shared" }).status, "granted", "shared 2 co-holds");
  eq(m.claim({ agent_id: "c", resource: "S", mode: "exclusive" }).status, "denied", "exclusive blocked by shared");
  const v = m.claims({ agent_id: "x" }).claims[0];
  eq(v.holder.sort(), ["a", "b"], "both shared holders listed");
});

test("lease expiry frees lock", ({ m, advance }) => {
  const g = m.claim({ agent_id: "a", resource: "R", lease_seconds: 10 });
  advance(20);
  const g2 = m.claim({ agent_id: "b", resource: "R" });
  eq(g2.status, "granted", "expired lease re-granted to b");
  ok(g2.fence > g.fence, "fence monotonic across expiry");
});

test("dead-agent claim self-heal", ({ m, advance }) => {
  m.register({ agent_id: "a", branch: "agent/a", prompt_ptr: "p", ttl_seconds: 60 });
  m.claim({ agent_id: "a", resource: "R", lease_seconds: 10000 });
  advance(200); // a is dead (60 + 30 grace)
  // b can grab it because a's claim is swept once a is dead.
  eq(m.claim({ agent_id: "b", resource: "R" }).status, "granted", "dead holder's lock freed");
});

test("not_holder vs unknown on release", ({ m }) => {
  const g = m.claim({ agent_id: "a", resource: "R" });
  eq(m.release({ agent_id: "b", claim_id: g.claim_id, resource: "R" }).status, "not_holder", "wrong agent = not_holder");
  eq(m.release({ agent_id: "a", claim_id: "ZZZ", resource: "R" }).status, "unknown", "bad claim_id = unknown");
  eq(m.release({ agent_id: "a", claim_id: "ZZZ", resource: "NOPE" }).status, "unknown", "missing resource = unknown");
});

// --- messaging -------------------------------------------------------------

test("durable mail, cursor exactly-once, privacy", ({ m }) => {
  const p = m.post({ agent_id: "a", to: "b", body: "hi" });
  eq(p.fanout, 1, "direct fanout = 1");
  eq(m.inbox({ agent_id: "c" }).messages.length, 0, "c does not see a->b (privacy)");
  const ib = m.inbox({ agent_id: "b" });
  eq(ib.messages.length, 1, "b sees the message");
  eq(ib.messages[0].body, "hi", "body delivered");
  eq(ib.unread, 0, "all returned, none unread");
  m.read({ agent_id: "b", up_to: p.message_id });
  eq(m.inbox({ agent_id: "b" }).messages.length, 0, "cursor consumes message exactly once");
});

test("broadcast and topic addressing", ({ m }) => {
  m.register({ agent_id: "a", branch: "x", prompt_ptr: "p" });
  m.register({ agent_id: "b", branch: "x", prompt_ptr: "p" });
  const bc = m.post({ agent_id: "a", to: "*", body: "all" });
  eq(bc.fanout, 2, "broadcast fanout = roster size");
  eq(m.inbox({ agent_id: "b" }).messages.length, 1, "b sees broadcast");
  m.post({ agent_id: "a", to: "topic:build", body: "t" });
  eq(m.inbox({ agent_id: "b" }).messages.filter((x) => x.to === "topic:build").length, 0, "no topic without subscription");
  eq(m.inbox({ agent_id: "b", topics: ["build"] }).messages.filter((x) => x.to === "topic:build").length, 1, "topic delivered to subscriber");
});

test("late joiner reads pending; ttl expiry + gc", ({ m, advance }) => {
  m.post({ agent_id: "a", to: "*", body: "early" });
  eq(m.inbox({ agent_id: "late" }).messages.length, 1, "late joiner sees prior broadcast");
  m.post({ agent_id: "a", to: "late", body: "ttl", ttl_seconds: 10 });
  eq(m.inbox({ agent_id: "late" }).messages.length, 2, "ttl message visible before expiry");
  advance(20);
  eq(m.inbox({ agent_id: "late" }).messages.length, 1, "ttl message hidden after expiry");
  const reclaimed = m.gc();
  eq(reclaimed, 1, "gc reclaims one expired message");
});

// --- cross-process durability ----------------------------------------------

test("state persists across daemon restarts", ({ m, dir }) => {
  const g = m.claim({ agent_id: "a", resource: "R" });
  // Fresh daemon over the same data dir resumes exactly where we left off.
  const m2 = new Mesh(join(dir, ".mesh"), { now: () => 1_000_000 });
  const v = m2.claims({ agent_id: "x" }).claims[0];
  eq(v.holder, ["a"], "reopened daemon sees prior claim");
  eq(v.claim_id, g.claim_id, "claim_id stable across restart");
});

console.log(passed + failed + " assertions: " + passed + " passed, " + failed + " failed");
process.exit(failed === 0 ? 0 : 1);
