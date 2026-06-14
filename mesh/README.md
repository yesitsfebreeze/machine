# mesh

Fleet inter-agent communication daemon — `kern`'s coordination sibling.

`kern` owns *memory* (why decisions were made). `mesh` owns *live coordination*:
who is here, who holds what, and who said what to whom. It is a per-cwd MCP
daemon that gives a fleet of parallel agents the four coordination primitives git
cannot cheaply provide: **awareness**, **atomic claims/locks**, **notify**, and
**peer-mesh chat**.

Implements SPEC-COMM-001.

## Install

```
cargo install --path .
```

This places a `mesh` binary on your `PATH` (same idiom as `kern`). The machine
plugin wires it in `.mcp.json`:

```json
"mesh": { "command": "mesh", "args": ["mcp"] }
```

## Run

```
mesh mcp        # MCP server over stdio (the fleet attaches here)
mesh gc         # reclaim TTL-expired messages and sweep dead claims
mesh --version
```

Data lives in a per-cwd, gitignored `.mesh/` directory:

- `data.mdb` / `lock.mdb` — LMDB primary store (roster, live claims, message bodies)
- `journal/history.db` (+ `-wal`/`-shm`) — SQLite-WAL journal (ordered message log,
  claim-event log, per-agent read cursors)

This mirrors `kern`'s `.kern/` shape exactly.

## The eight verbs

All are MCP tools on the `mesh` server, namespaced `mcp__mesh__*`. Every request
carries the caller's `agent_id` (the git-fs `agent/<id>` identity), treated as the
authenticated principal. This is the verb summary only — see SPEC-COMM-001 §4 for
the full request/response shapes (the single source of truth for field names and
meanings).

| Family | Verb | Purpose |
|---|---|---|
| awareness | `register` | Announce presence / heartbeat (idempotent upsert). |
| awareness | `roster` | List known agents and their liveness. |
| claims | `claim` | Atomically acquire (or queue for) a resource lock. |
| claims | `release` | Relinquish a held claim or cancel a queued ticket. |
| claims | `claims` | Inspect current locks and queues. |
| messaging | `post` | Send a durable message (`agent_id`, `*`, or `topic:<name>`). |
| messaging | `inbox` | Peek pending messages without advancing the cursor. |
| messaging | `read` | Advance the read cursor (acknowledge consumption). |

## Atomic claims

The core value. Every grant, release, expiry, and queue promotion happens inside
a single LMDB write transaction — the cross-process compare-and-swap git cannot
provide. Under concurrent contention for an `exclusive` resource, exactly one
caller wins; the rest are `denied` or `queued`.

- **Modes:** `exclusive` (default) and `shared` (co-holders allowed).
- **Wait policy:** `no_wait` (fail fast) or `queue` (enqueue, promoted FIFO on
  release/expiry).
- **Fence token:** monotonically increasing per resource across its whole life,
  even after the lock is fully released and re-acquired.
- **Self-healing:** a crashed holder's lock is freed on lease expiry, and a dead
  agent's claims are released once its liveness reaches `dead`. Liveness follows
  the SPEC R-3 state machine: `alive` while within the heartbeat TTL, `stale`
  during a bounded grace window after it, then `dead`. Only the `dead` state frees
  claims — a `stale` agent keeps its holds — so no lock is held forever.

## Messaging

Durable mail that survives sender death and reaches recipients that did not exist
at post time. A broadcast/topic message is stored once; each recipient reads it
through its own cursor, so a fleet-wide broadcast does not multiply storage by
fleet size. Topic subscription is expressed by the reader at poll time, so a
late-joining agent can read a topic's pending messages.

## Delivery

The API surface is storage-agnostic about delivery (poll vs hub-relay). Delivery
is purely a consumption concern handled through `inbox`/`read` and their cursors;
the same verbs serve both the cooperative-poll model and an orchestrator
hub-relay, with no data migration between them.

## Tests

```
cargo test
```

The acceptance suite covers the highest-risk SPEC criteria: atomic exclusive
grant under concurrent contention, queue promotion on release, lease-expiry and
dead-agent self-heal, durable mail with exactly-once delivery via cursor, mail and
cursor privacy, broadcast/topic semantics, and the storage shape on disk.
