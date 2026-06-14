# Changelog

All notable changes to the `mesh` daemon are documented here.

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
