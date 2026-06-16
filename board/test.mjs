#!/usr/bin/env node
// Acceptance suite for the board daemon. Zero deps; run: `node board/test.mjs`.
// Exercises the domain verbs against a temp BOARD_DIR (mirrors mesh/test.mjs).

import { Board } from "./board.mjs";
import { mkdtempSync, rmSync, readFileSync } from "node:fs";
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

function fixture() {
  const dir = mkdtempSync(join(tmpdir(), "board-test-"));
  // Isolate the global cwd index per fixture so project_resolve's self-register
  // never touches the real ~/.local/share index, and cleans up with the temp dir.
  const indexFile = join(dir, "index.json");
  process.env.BOARD_INDEX = indexFile;
  const revs = [];
  const b = new Board(join(dir, ".board"), { onMutate: (r) => revs.push(r) });
  return { b, dir, indexFile, revs };
}

function test(name, fn) {
  const f = fixture();
  try { fn(f); }
  catch (e) { failed++; console.error(`  THREW in ${name}: ${e.stack || e}`); }
  finally { rmSync(f.dir, { recursive: true, force: true }); }
}

// --- projects --------------------------------------------------------------

test("project_resolve is get-or-create by name; project_list ordered", ({ b }) => {
  const p1 = b.project_resolve({ name: "machine" }).project;
  ok(p1.id && p1.name === "machine", "first resolve creates");
  const p2 = b.project_resolve({ name: "machine" }).project;
  eq(p2.id, p1.id, "second resolve returns same project (no dup)");
  const p3 = b.project_resolve({ name: "other" }).project;
  ok(p3.id !== p1.id, "different name -> different project");
  eq(b.project_list().projects.length, 2, "two distinct projects");
});

// --- global cwd index (singleton aggregation) ------------------------------

test("project_resolve self-registers this repo's board dir in the global index", ({ b, indexFile }) => {
  b.project_resolve({ name: "machine" });
  const idx = JSON.parse(readFileSync(indexFile, "utf8"));
  const entry = idx.boards[b.root];
  ok(entry, "index has an entry keyed by repo root");
  eq(entry.dir, b.dir, "entry points at this repo's .board dir");
  eq(entry.name, "machine", "entry carries the project/cwd name");
});

test("re-resolving the same cwd does not duplicate index entries", ({ b, indexFile }) => {
  b.project_resolve({ name: "machine" });
  b.project_resolve({ name: "machine" });
  const idx = JSON.parse(readFileSync(indexFile, "utf8"));
  eq(Object.keys(idx.boards).length, 1, "one cwd -> one index entry");
});

test("two repos sharing one index both appear (dropdown aggregation)", ({ dir, indexFile }) => {
  const a = new Board(join(dir, "repoA", ".board"));
  const z = new Board(join(dir, "repoZ", ".board"));
  a.project_resolve({ name: "alpha" });
  z.project_resolve({ name: "zeta" });
  const idx = JSON.parse(readFileSync(indexFile, "utf8"));
  eq(Object.keys(idx.boards).length, 2, "two distinct cwds registered");
  ok(idx.boards[a.root] && idx.boards[z.root], "both cwds present in the index");
});

// --- columns + board_get ---------------------------------------------------

test("columns created left-to-right; board_get groups by sort", ({ b }) => {
  const p = b.project_resolve({ name: "p" }).project;
  const todo = b.column_create({ projectId: p.id, name: "todo" }).column;
  const doing = b.column_create({ projectId: p.id, name: "doing" }).column;
  eq(todo.sort, 0, "first column sort 0");
  eq(doing.sort, 1, "second column sort 1");
  const board = b.board_get({ projectId: p.id });
  eq(board.columns.map((c) => c.name), ["todo", "doing"], "columns left-to-right");
  eq(board.columns.map((c) => c.cards.length), [0, 0], "columns start empty");
});

test("board_get rejects unknown project", ({ b }) => {
  let threw = false;
  try { b.board_get({ projectId: "NOPE" }); } catch { threw = true; }
  ok(threw, "unknown project throws");
});

// --- cards -----------------------------------------------------------------

test("card create/update; board_get carries cards + comment counts", ({ b }) => {
  const p = b.project_resolve({ name: "p" }).project;
  const col = b.column_create({ projectId: p.id, name: "todo" }).column;
  const k = b.card_create({ columnId: col.id, title: "[grilling] x", body: "e1" }).card;
  eq(k.title, "[grilling] x", "card title set");
  eq(k.body, "e1", "card body set");
  b.card_update({ id: k.id, title: "[planning] x" });
  const board = b.board_get({ projectId: p.id });
  eq(board.columns[0].cards[0].title, "[planning] x", "update reflected");
  eq(board.columns[0].cards[0].commentCount, 0, "comment count starts 0");
});

test("card_move across columns lands at requested index and reorders", ({ b }) => {
  const p = b.project_resolve({ name: "p" }).project;
  const a = b.column_create({ projectId: p.id, name: "a" }).column;
  const z = b.column_create({ projectId: p.id, name: "z" }).column;
  const k1 = b.card_create({ columnId: z.id, title: "k1" }).card;
  const k2 = b.card_create({ columnId: z.id, title: "k2" }).card;
  const moved = b.card_create({ columnId: a.id, title: "moved" }).card;
  b.card_move({ id: moved.id, toColumnId: z.id, newIndex: 1 });
  const board = b.board_get({ projectId: p.id });
  const zcol = board.columns.find((c) => c.name === "z");
  eq(zcol.cards.map((c) => c.title), ["k1", "moved", "k2"], "moved card lands at index 1");
  const acol = board.columns.find((c) => c.name === "a");
  eq(acol.cards.length, 0, "source column now empty");
});

test("card_delete removes the card and its comments", ({ b }) => {
  const p = b.project_resolve({ name: "p" }).project;
  const col = b.column_create({ projectId: p.id, name: "c" }).column;
  const k = b.card_create({ columnId: col.id, title: "k" }).card;
  b.comment_add({ cardId: k.id, author: "drill", body: "note" });
  b.card_delete({ id: k.id });
  const board = b.board_get({ projectId: p.id });
  eq(board.columns[0].cards.length, 0, "card gone");
  eq(b.comment_list({ cardId: k.id }).comments.length, 0, "comments gone");
});

test("column_delete cascades to cards and comments", ({ b }) => {
  const p = b.project_resolve({ name: "p" }).project;
  const col = b.column_create({ projectId: p.id, name: "c" }).column;
  const k = b.card_create({ columnId: col.id, title: "k" }).card;
  b.comment_add({ cardId: k.id, author: "a", body: "x" });
  b.column_delete({ id: col.id });
  eq(b.board_get({ projectId: p.id }).columns.length, 0, "column gone");
  eq(b.comment_list({ cardId: k.id }).comments.length, 0, "cascaded comments gone");
});

// --- comments --------------------------------------------------------------

test("comments persist, list oldest-first, and bump card comment count", ({ b }) => {
  const p = b.project_resolve({ name: "p" }).project;
  const col = b.column_create({ projectId: p.id, name: "c" }).column;
  const k = b.card_create({ columnId: col.id, title: "k" }).card;
  b.comment_add({ cardId: k.id, author: "u1", body: "first" });
  b.comment_add({ cardId: k.id, author: "u2", body: "second" });
  const list = b.comment_list({ cardId: k.id }).comments;
  eq(list.map((c) => c.body), ["first", "second"], "comments oldest-first");
  eq(b.board_get({ projectId: p.id }).columns[0].cards[0].commentCount, 2, "comment count = 2");
});

// --- rev + live-push hook --------------------------------------------------

test("every mutation bumps rev and fires onMutate; reads do not", ({ b, revs }) => {
  const p = b.project_resolve({ name: "p" }).project; // mutation 1
  b.project_list();                                   // read, no bump
  b.column_create({ projectId: p.id, name: "c" });    // mutation 2
  eq(revs, [1, 2], "two mutations -> revs [1,2], reads silent");
});

// --- cross-process durability ----------------------------------------------

test("state persists across daemon restarts", ({ b, dir }) => {
  const p = b.project_resolve({ name: "p" }).project;
  const col = b.column_create({ projectId: p.id, name: "c" }).column;
  b.card_create({ columnId: col.id, title: "survives" });
  const b2 = new Board(join(dir, ".board"));
  const board = b2.board_get({ projectId: p.id });
  eq(board.columns[0].cards[0].title, "survives", "reopened daemon sees prior card");
});

console.log(passed + failed + " assertions: " + passed + " passed, " + failed + " failed");
process.exit(failed === 0 ? 0 : 1);
