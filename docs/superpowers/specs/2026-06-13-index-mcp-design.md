# Index MCP + `/doctor` — Design Spec

**Date:** 2026-06-13
**Status:** Approved (brainstorm) — pending implementation plan
**Topic:** A central, live file-index MCP that all code-file resolution flows through, plus a `/doctor` machine self-audit umbrella that consumes it.

## Problem

The machine enforces several laws by eyeball that should be enforced by a tool:
- "Each fact lives once" / "one clean implementation" — no prose-duplication detector across the 87 instruction files.
- Broken `@import` and named skill/agent cross-references rot silently at this scale.
- `/improve` re-globs and re-measures the whole tree every run; nothing shares that work.
- Nothing audits the **machine itself** for the laws it claims to enforce (`/bootstrap` seeds, `/gate` checks code).

Root cause: there is no single, authoritative, fast source of "what files exist and what's in them." Every consumer re-discovers the file set independently.

## Solution overview

A small **`index` MCP** — a running stdio service that maintains a live file index via an fs watcher — becomes the central substrate. All code-file resolution flows through it. Two consumers ship first (`/improve`, `/doctor`); the default agent's resolution is routed through it with a Glob fallback.

```
.mcp.json -> starts `index` MCP (node .claude/mcp/index/server.mjs)
                     |
        +------------+--------------------------+
   fs watcher    in-mem index            snapshot -> .proj/index.json
   (debounced,   {path,ext,lines,           (warm cold-start; watcher
    .gitignore-   bytes,mtimeMs,hash}        keeps it fresh on the fly)
    aware)
                     |  exposes tools v
   index_resolve . index_list . index_stat . index_dupes . index_stale
                     |  consumed by v
        /improve   .   /doctor   .   default agent (code-file resolution)
```

## Components

### 1. `index` MCP server (new)

Location: `.claude/mcp/index/` — `server.mjs` (JSON-RPC loop), `indexer.mjs` (build/patch index), `watcher.mjs` (fs events).

**Protocol:** dependency-free stdio **JSON-RPC 2.0**. Implement only the MCP methods needed by hand over stdin/stdout line framing — `initialize`, `tools/list`, `tools/call`. NO `@modelcontextprotocol/sdk`, NO npm dependencies. This preserves the machine idiom (personas/statusline hooks are dependency-free `.mjs`; `kern` is an external binary; there is no compile step and no `node_modules`).

**Wiring:** add to `.mcp.json` alongside `kern`:
```json
"index": { "command": "node", "args": [".claude/mcp/index/server.mjs"] }
```

### 2. Index model

Per-file record:
```
{ path, ext, lines, bytes, mtimeMs, hash, shingles? }
```
- `hash` = SHA-1 of content with line-endings normalized (CRLF -> LF). The repo is Windows; CRLF vs LF must NOT register as a change.
- `shingles` computed lazily — only for `.md` and source files, only when `index_dupes` is called. Not stored hot for every file (pay-back-context discipline).
- Initial build seeds from `git ls-files` (respects `.gitignore`, faster than `find`).

### 3. Watcher

`fs.watch(repo, {recursive:true})` — native recursive on Windows. Debounced ~200ms. Changes filtered through `.gitignore`. On each event: re-stat + re-hash only touched files, patch the in-mem index, throttle-write the snapshot.

### 4. Snapshot persistence

Snapshot to `.proj/index.json` for instant cold start; watcher keeps it warm. **Atomic write:** write `.proj/index.json.tmp` then rename, so a crash mid-write never corrupts the index.

### 5. `/sweep` skill (new, thin control surface)

The daemon does continuous work, so `/sweep` is only the manual override + health view:
- `rebuild` — force a full rescan.
- `health` — file count, last-update time, watcher-alive, staleness vs HEAD.

### 6. `/doctor` umbrella skill (new)

Reads the index (triggers `/sweep rebuild` if stale/missing), runs check modules, prints a pass/fail table like `/gate`.

| Module | Build now? | Checks |
|---|---|---|
| broken-refs | yes | every `@import` + named skill/agent cross-ref resolves to a live file; flags refs to retired siblings. Walks `index_list('.md')`. |
| prose-dup | yes | near-duplicate passages via `index_dupes` over the `.md` subset. |
| size-limit | stub slot | CLAUDE.md <= 40k chars. |
| language-policy | stub slot | non-English in instruction docs. |
| orphan-todo | stub slot | TODO/FIXME/`unwrap()`/`panic!` with git-blame age. |

Stubs are named slots in the skill doc, not code — cheap to fill later.

### 7. `/improve` integration

One edit, no rewrite: scope resolves via `index_resolve`; duplication category via `index_dupes`; re-rate set via `index_stale`. No re-globbing.

### 8. Default-agent integration

Edit `default.md`: prefer `index_resolve` for code-file resolution, **fall back to Glob if the MCP is unreachable**. No hard dependency.

## Tool I/O contracts

| Tool | In | Out |
|---|---|---|
| `index_resolve` | `{target: glob\|ext\|dir}` | `{paths: string[]}` |
| `index_list` | `{ext?, dir?}` | `{files: Record[]}` |
| `index_stat` | `{path}` | `{record}` or `{error:"not-indexed"}` |
| `index_dupes` | `{scope?, k?:lines, minOverlap?}` | `{pairs:[{a,b,ranges,overlap}]}` |
| `index_stale` | `{sinceSha}` | `{changed:string[], deleted:string[]}` |

## Prose-dup mechanics

Normalize -> split into k-line shingles -> hash each -> any shingle hash colliding across two files is a candidate -> report file pairs + line ranges, ranked by overlap volume. Deterministic, zero model tokens, hook-able.

## Degradation contract (machine law: never a silent break)

- MCP unreachable -> consumers fall back to Glob / `git ls-files`, and **say so** in output.
- Watcher dies but server lives -> `index_stat`/`resolve` still answer from snapshot; `/sweep health` reports `watcher: dead`.
- Snapshot write is atomic (tmp + rename) -> a crash mid-write never corrupts the index.

## Why a daemon is justified

Three consumers share it (improve, doctor, default-agent resolution) and the watcher removes per-run scan cost. If it were one consumer, static-JSON `/sweep` would be the right call — that remains the cheap fallback if the daemon proves not worth it.

Concrete payoff beyond dup-detection: `.proj/project.md` is already stale ("21 skill dirs / 16 language rules / thin-command pattern" vs. 87 skill files, deleted `languages/`, retired `commands/`). `/doctor` over the live index catches exactly this drift.

## Out of scope

- kern integration (explicitly excluded by user).
- skill-usage telemetry / dead-skill detection (needs transcript analysis — separate effort).
- settings-vs-CLI-version validator (separate `/doctor` module, later).

## Build order

1. `index` MCP server (server + indexer + watcher) + `.mcp.json` wiring.
2. `/sweep` control skill.
3. `/doctor` umbrella + broken-refs + prose-dup modules.
4. `/improve` integration edit.
5. `default.md` resolution-routing edit (with Glob fallback).
