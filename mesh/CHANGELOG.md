# Changelog

All notable changes to the `mesh` daemon are documented here.

## [0.3.0] - 2026-06-16

Repo-scoped store. The mesh state directory is now resolved at the repository
root (the parent of git's common dir) instead of `process.cwd()`, so every git
worktree of one repository shares a single roster/claims/messages namespace.
This is the invariant the worktree-isolated fleet needs: a drill driver and its
miners, each in its own worktree, now see one another on the roster.

### Changed

- `dataDir()` resolution order: `MESH_DIR` env override, then the repo root via
  `git rev-parse --git-common-dir`, then `process.cwd()` as the fallback when not
  inside a git repository. Previously the store was hard-wired to
  `join(process.cwd(), ".mesh")`, which gave each worktree an isolated store and
  silently broke cross-worktree coordination.

## [0.2.0] - 2026-06-15

Zero-dependency Node ESM rewrite. The daemon is now a single self-contained
`mesh.mjs` script requiring only a Node runtime — no Rust toolchain, no `cargo`
build, no native modules.

### Changed

- Replaced the Rust + LMDB + SQLite implementation with one Node ESM file.
  Launches as `node mesh.mjs mcp`; wired in `.mcp.json` via `${CLAUDE_PLUGIN_ROOT}`.
- Storage is a single JSON document (`.mesh/state.json`). Cross-process atomicity
  for the claim CAS comes from an OS-atomic lock directory plus atomic rename on
  write; a stale lock from a crashed process is reclaimed after a bounded wait.

### Preserved

- Identical wire contract: the same eight verbs (`register`, `roster`, `claim`,
  `release`, `claims`, `post`, `inbox`, `read`) with the same request/response
  shapes, and the same semantics — exclusive/shared modes, no_wait/queue policies,
  monotonic per-resource fence, lease expiry and dead-agent self-heal, durable
  exactly-once mail via per-agent cursors, broadcast/topic addressing.
- Acceptance suite (`node test.mjs`) covering the highest-risk SPEC criteria.

## [0.1.0] - 2026-06-14

Initial implementation of SPEC-COMM-001.

### Added

- Single native binary `mesh` whose MCP server launches as `mesh mcp` over stdio,
  mirroring `kern mcp`.
- Eight MCP verbs (`register`, `roster`, `claim`, `release`, `claims`, `post`,
  `inbox`, `read`) exposed as `mcp__mesh__*`, with request/response shapes per
  SPEC §4.
- Storage: LMDB primary store (roster, live claims, message bodies) plus a
  SQLite-WAL journal (ordered message log, claim-event log, per-agent read
  cursors), in a per-cwd gitignored `.mesh/` data directory.
- Atomic claims: `exclusive`/`shared` modes, `no_wait`/`queue` wait policies, a
  monotonic per-resource fence token, lease expiry and dead-agent self-heal, and
  FIFO queue promotion on release/expiry — all within single LMDB write
  transactions.
- Durable messaging: point-to-point, `*` broadcast, and `topic:<name>` addressing;
  messages survive sender death and reach late-joining recipients; stored once
  with per-recipient read cursors; `ttl_seconds` and `reply_to` threading.
- Per-`agent_id` trust: only a holder releases/renews its claim; only a recipient
  reads its own mail and advances its own cursor.
- `mesh gc` maintenance command (TTL-expired message reclamation + dead-claim
  sweep).
- Plugin wiring: `mesh` added to the machine's `.mcp.json`; `.mesh/` added to
  `.gitignore`.
