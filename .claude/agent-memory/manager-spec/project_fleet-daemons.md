---
name: project-fleet-daemons
description: The machine's fleet infra — kern memory daemon shape, the planned mesh coordination daemon (SPEC-COMM-001), and git-fs as a separate companion plugin
metadata:
  type: project
---

The machine fleet uses sibling per-cwd MCP daemons plus an external git plugin.

- **kern** (memory/knowledge graph): Rust native binary at `~/.cargo/bin/kern`,
  launched as `kern mcp` over stdio, wired in repo-root `.mcp.json`. Storage shape:
  **LMDB primary** (`.kern/data.mdb` + `lock.mdb`) plus a **SQLite-WAL journal**
  (`.kern/journal/history.db` + `-wal`/`-shm`). `.kern/` is gitignored. Tools are
  `mcp__kern__*`. Has a gossip `peers` model (cross-store federation for memory).
- **mesh** (coordination — SPEC'd, not yet built): see SPEC-COMM-001 at
  `/mnt/e/dev/machine/.machine/specs/comm-daemon/SPEC.md`. kern's sibling for
  live coordination: roster, atomic claims/locks, durable messages. Mirrors kern's
  language/storage/lifecycle. Owns ONLY dynamic coordination state, never code or chat-in-git.
- **git-fs** is a SEPARATE companion plugin (`yesitsfebreeze/git-fs`), NOT bundled by
  the machine and NOT a daemon — each agent on branch `agent/<id>`, every edit a
  commit, Stop hook merges to main; carries a per-agent `.machine-prompt` intent file.

**Why:** these are the binding facts for any coordination/fleet SPEC — what each layer
owns, so SPECs don't duplicate responsibilities. **How to apply:** when SPECing fleet
features, match kern's daemon pattern (Rust binary, LMDB+SQLite-WAL, per-cwd, stdio MCP,
gitignored data dir); keep code/history/intent in git-fs; keep coordination in mesh;
keep memory in kern. Verify the kern binary/store still exist before relying on specifics.
