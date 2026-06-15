# mesh

Fleet inter-agent communication daemon — `kern`'s coordination sibling.

`kern` owns *memory* (why decisions were made). `mesh` owns *live coordination*:
who is here, who holds what, and who said what to whom. It is a per-cwd MCP
daemon that gives a fleet of parallel agents the four coordination primitives git
cannot cheaply provide: **awareness**, **atomic claims/locks**, **notify**, and
**peer-mesh chat**.

Implements SPEC-COMM-001.

## Zero dependencies

`mesh` is a single self-contained Node ESM script (`mesh.mjs`). It needs nothing
but a Node runtime — no build step, no `cargo`, no npm install, no native
modules. The machine already requires Node (for `context-mode`), so `mesh` adds
no new dependency.

## Run

```
node mesh.mjs mcp        # MCP server over stdio (the fleet attaches here)
node mesh.mjs gc         # reclaim TTL-expired messages and sweep dead claims
node mesh.mjs --version
```

The machine plugin wires it in `.mcp.json` so no manual launch is needed:

```json
"mesh": { "command": "node", "args": ["${CLAUDE_PLUGIN_ROOT}/mesh/mesh.mjs", "mcp"] }
```

## Storage

State lives in a per-cwd, gitignored `.mesh/` directory:

- `state.json` — the single JSON document holding the roster, live claims, message
  bodies, the ordered message log, the claim-event log, and per-agent read cursors.
- `.lock/` — a short-lived OS-atomic lock directory held only for the duration of
  one mutating operation.

Cross-process atomicity (the compare-and-swap the claim primitive needs) comes
from the lock directory: `mkdir` is atomic across processes, so when two agents
race for the same exclusive resource exactly one wins. Each write is committed
with an atomic rename, so a crash never leaves a half-written state file. A stale
lock left by a crashed process is reclaimed after a bounded wait.

## The eight verbs

All are MCP tools on the `mesh` server, namespaced `mcp__mesh__*`. Every request
carries the caller's `agent_id` (the git-fs `agent/<id>` identity), treated as the
authenticated principal.

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

The core value. Every grant, release, expiry, and queue promotion happens under
the lock as one read-modify-write of the state file — the cross-process CAS git
cannot provide. Under concurrent contention for an `exclusive` resource, exactly
one caller wins; the rest are `denied` or `queued`.

- **Modes:** `exclusive` (default) and `shared` (co-holders allowed).
- **Wait policy:** `no_wait` (fail fast) or `queue` (enqueue, promoted FIFO on
  release/expiry).
- **Fence token:** monotonically increasing per resource across its whole life,
  even after the lock is fully released and re-acquired.
- **Self-healing:** a crashed holder's lock is freed on lease expiry, and a dead
  agent's claims are released once its liveness reaches `dead`. Liveness: `alive`
  within the heartbeat TTL, `stale` during a bounded grace window after it, then
  `dead`. Only `dead` frees claims — a `stale` agent keeps its holds — so no lock
  is held forever.

## Messaging

Durable mail that survives sender death and reaches recipients that did not exist
at post time. A broadcast/topic message is stored once; each recipient reads it
through its own cursor, so a fleet-wide broadcast does not multiply storage by
fleet size. Topic subscription is expressed by the reader at poll time, so a
late-joining agent can read a topic's pending messages.

## Tests

```
node test.mjs
```

The acceptance suite covers the highest-risk SPEC criteria: atomic exclusive
grant under concurrent contention, queue promotion on release, lease-expiry and
dead-agent self-heal, durable mail with exactly-once delivery via cursor, mail and
cursor privacy, broadcast/topic semantics, and state durability across restarts.
