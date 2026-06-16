---
id: SPEC-COMM-001
title: "Fleet inter-agent communication daemon (mesh) — kern's coordination sibling"
version: 1.0.0
status: draft
created: 2026-06-14
updated: 2026-06-14
author: machine-manager-spec
priority: high
issue_number: null
---

# Fleet Inter-Agent Communication Daemon (working name: `mesh`)

A per-cwd MCP daemon that gives a fleet of parallel agents the four coordination
primitives git cannot cheaply provide: **awareness**, **atomic claims/locks**,
**notify**, and **peer-mesh chat**. It is `kern`'s sibling: `kern` owns *memory*
(why decisions were made), `mesh` owns *live coordination* (who is here, who holds
what, who said what to whom). It is shipped as a new MCP server bundled by the
machine plugin, matching `kern`'s structure, language, storage, and lifecycle.

This document specifies WHAT and WHY in EARS form. It does not prescribe function
names, struct layouts, or wire encodings beyond the request/response *shapes* the
contract requires.

## HISTORY

| Version | Date | Author | Change |
|---------|------|--------|--------|
| 1.0.0 | 2026-06-14 | machine-manager-spec | Initial draft. Specifies the two-layer split against git-fs, the ~8-verb contract (register/roster, claim/release/claims, post/inbox/read), atomic-claim semantics with staleness/expiry, the message model (addressing, topics, broadcast, durability, read cursors), the cooperative-poll-primary + hub-relay-alternative delivery resolution, the LMDB-primary + SQLite-journal storage decision matching kern, the roster/liveness model, the trust model, and the numbered acceptance criteria. |

---

## 1. Problem statement

### 1.1 The setting

The machine runs many agents in parallel across one repo. Two pieces of infra
already exist:

- **git-fs** — each agent works on branch `agent/<id>`; every edit is a commit; a
  Stop hook merges the branch to `main`. This gives **isolation** (agents do not
  stomp each other's working tree) and a durable **change history** (every edit is
  recoverable). git-fs also carries one per-agent `.machine-prompt` file: the
  agent's **declared intent** for its branch.
- **kern** — a per-cwd MCP daemon that remembers *why* past decisions were made
  (knowledge graph + memory), recalled at session start.

### 1.2 What is missing

git gives change history but is the wrong tool for *live, cross-branch
coordination*:

- **No cheap atomic CAS across branches.** Two agents on two branches cannot use
  git to atomically agree "I, and only I, am editing module X right now." A commit
  on branch A is invisible to branch B until a merge, and a merge is not an atomic
  lock-acquire. There is no compare-and-swap primitive that is correct *before*
  the work starts. This is the single highest-value gap.
- **No roster.** An agent cannot ask "who else is alive in this repo right now and
  what are they each trying to do?" without scraping branches and guessing
  liveness from commit timestamps.
- **No durable point-to-point or broadcast messaging.** Putting chat into git
  commits pollutes the change history with non-code content, couples message
  delivery to merge timing, and has no read-cursor semantics. Messages must
  survive the *death of the sending agent* and be deliverable to an agent that was
  not alive when the message was posted.

### 1.3 Why a daemon, and why a sibling of kern

These are *ephemeral, high-churn, cross-process* facts (presence, locks, mail) —
exactly the class of state a long-lived per-cwd daemon serves well and git serves
badly. `kern` already proves the pattern in this repo: a per-cwd daemon reached
over MCP, with a binary primary store and a journal. `mesh` reuses that pattern so
the fleet has one coherent operational story.

---

## 2. Two-layer architecture (binding — do not re-litigate)

The fleet's state is split across two owners. Each fact lives in exactly one
layer (single source of truth).

| Concern | Owner | Rationale |
|---|---|---|
| Code, working tree, every edit as a commit | **git-fs** | Isolation + recoverable change history. |
| Per-agent **declared intent** (`.machine-prompt`) | **git-fs** | Intent is versioned with the branch it describes. |
| **Roster** (who is alive, liveness, pointer to each agent's intent) | **mesh** | Presence is live and ephemeral; it must not be a commit. |
| **Claims / locks** (atomic resource ownership) | **mesh** | Requires cross-branch atomic CAS git cannot give. |
| **Messages** (point-to-point, broadcast, topics) | **mesh** | Durable mail with read cursors; never code, never a commit. |

### 2.1 Hard boundaries

- The SPEC'd system **shall not** store any agent chat, message, claim record, or
  roster entry inside a git commit. *(Why: keeps the change history pure code and
  decouples coordination from merge timing.)*
- The SPEC'd system **shall not** duplicate any git-fs responsibility — it shall
  not version code, shall not own the working tree, and shall not store the
  declared-intent text. For intent, the roster shall hold only a **pointer**
  (branch + path) to the git-fs-owned `.machine-prompt`, never a copy.
- `mesh` and `kern` shall be separate daemons with separate data directories.
  `mesh` shall not write into `kern`'s store and vice versa. *(Why: coordination
  churn must not perturb the memory graph; the two have different durability and
  GC characteristics.)*

### EARS — architecture

- **A-1.** The system **shall** persist exactly three categories of dynamic
  coordination state: roster entries, claims, and messages.
- **A-2.** The system **shall** store a roster entry's reference to declared
  intent as a pointer (branch name + file path) to the git-fs-owned
  `.machine-prompt`, and **shall not** store the intent text itself.
- **A-3.** **If** a write would place coordination state (roster, claim, or
  message) into a git commit, **then** the system **shall** reject that path —
  coordination state lives only in the daemon store.
- **A-4.** The system **shall** operate as a process distinct from `kern`, with a
  distinct per-cwd data directory.

---

## 3. Daemon, language, and storage decisions

### 3.1 Language — match kern (Rust)

`kern` ships as a Cargo-installed native binary (`kern`) invoked as `kern mcp`. To
keep the fleet's operational story coherent (one install idiom, one daemon
lifecycle model, shared concurrency and LMDB ecosystem), `mesh` shall be
implemented in the same language as `kern` (Rust) and shipped the same way: a
single native binary whose MCP server is launched as a subcommand over stdio.

- **L-1.** The system **shall** be distributed as a single native binary in the
  same language and install idiom as `kern`.
- **L-2.** The system **shall** expose its MCP server over stdio, launched as a
  subcommand of that binary (mirroring `kern mcp`).

### 3.2 Storage — LMDB primary + SQLite-WAL journal (match kern)

`kern`'s on-disk shape is an LMDB primary store (`data.mdb` / `lock.mdb`) plus a
SQLite-WAL journal (`journal/history.db` with `-wal`/`-shm`). `mesh` adopts the
same split, for these reasons:

- **Atomic claims demand real ACID CAS.** The claim primitive is a
  compare-and-swap under contention from multiple OS processes (each agent is its
  own process). LMDB gives a single-writer, MVCC, fully-ACID transaction with a
  process-shared lock file — a claim acquire is one LMDB write transaction that
  either wins or loses atomically. This is precisely the primitive git lacks and
  the reason a key-value transactional store beats SQLite-as-primary for the hot
  claim path (no SQL parse/plan on the contended path, lock-free readers).
- **Consistency with kern.** Same store family means shared backup, compaction
  (`compact`/`gc`), and operational knowledge already documented for `kern`.
- **The journal answers durability + audit.** Messages and claim transitions need
  an append-only, queryable, time-ordered log (for inbox replay, read-cursor
  recovery, and post-mortem "who held X when"). SQLite-WAL is the right shape for
  range/time queries and survives crashes; it mirrors `kern`'s `history.db`.

Decision: **LMDB is the primary store** (roster, live claims, message bodies keyed
for O(1) fetch); **SQLite-WAL is the journal** (ordered message log, claim-event
log, read cursors). This is the single most consequential storage decision and it
defaults toward kern's choice deliberately.

- **S-1.** The system **shall** use an LMDB primary store for roster, live claims,
  and message records.
- **S-2.** The system **shall** acquire and release claims within a single LMDB
  write transaction so that acquisition is atomic across concurrent OS processes.
- **S-3.** The system **shall** maintain a SQLite-WAL journal as an append-only,
  time-ordered log of message and claim-transition events, and of per-agent read
  cursors.
- **S-4.** The system **shall** place its data under a per-cwd data directory that
  is excluded from version control (mirroring how `.kern/` is gitignored).
- **S-5.** **Where** the store grows unbounded, the system **shall** provide a
  compaction/GC path (mirroring kern's `compact`/`gc`) that reclaims space without
  losing live claims or unread messages.

---

## 4. Verb / API contract

Eight verbs in three families. All verbs are MCP tools on the `mesh` server,
namespaced `mcp__mesh__*` (mirroring `mcp__kern__*`). Every request carries the
caller's `agent_id` (the git-fs `agent/<id>` identity); the daemon treats
`agent_id` as the authenticated principal for the trust rules in §8.

Shapes below are *contracts* (fields + meaning), not serialization formats.
Timestamps are ISO-8601 UTC. `ulid` denotes a sortable unique id.

### 4.1 Family: awareness — `register`, `roster`

#### `register` — announce presence and refresh liveness

Used both to join the fleet and as the periodic heartbeat (idempotent upsert
keyed by `agent_id`).

Request:
| field | type | meaning |
|---|---|---|
| `agent_id` | string | The agent's `agent/<id>` identity. |
| `branch` | string | The git-fs branch the agent works on. |
| `prompt_ptr` | string | Path to the git-fs `.machine-prompt` (pointer, not text). |
| `role` | string? | Optional declared role/specialty (e.g. `expert-backend`). |
| `ttl_seconds` | int? | Liveness window; default applies if omitted. |

Response:
| field | type | meaning |
|---|---|---|
| `agent_id` | string | Echoed. |
| `registered_at` | ts | First-seen time (stable across heartbeats). |
| `expires_at` | ts | `now + ttl`; presence is stale after this. |
| `epoch` | int | Incremented each fresh join; lets peers detect a restart. |

#### `roster` — list known agents and their liveness

Request:
| field | type | meaning |
|---|---|---|
| `agent_id` | string | Caller. |
| `include_stale` | bool? | Include agents past `expires_at` (default false). |

Response:
| field | type | meaning |
|---|---|---|
| `agents` | list | One entry per known agent. |
| `agents[].agent_id` | string | Identity. |
| `agents[].branch` | string | git-fs branch. |
| `agents[].prompt_ptr` | string | Pointer to declared intent (resolve via git-fs). |
| `agents[].role` | string? | Declared role. |
| `agents[].liveness` | enum | `alive` \| `stale` \| `dead`. |
| `agents[].last_seen` | ts | Last heartbeat. |
| `agents[].expires_at` | ts | Liveness deadline. |
| `agents[].held_claims` | list | Resource ids this agent currently holds. |

### 4.2 Family: atomic claims/locks — `claim`, `release`, `claims`

This is the core value: a cross-process atomic resource lock.

#### `claim` — atomically acquire (or queue for) a resource lock

Request:
| field | type | meaning |
|---|---|---|
| `agent_id` | string | Caller. |
| `resource` | string | Opaque lock key (agents agree the namespace, e.g. a path). |
| `mode` | enum | `exclusive` (default) \| `shared`. |
| `lease_seconds` | int? | Auto-expiry of the hold; default applies if omitted. |
| `wait` | enum | `no_wait` (fail fast) \| `queue` (enqueue if held). |
| `note` | string? | Human-readable reason, surfaced in `claims`. |

Response:
| field | type | meaning |
|---|---|---|
| `status` | enum | `granted` \| `queued` \| `denied`. |
| `resource` | string | Echoed. |
| `claim_id` | ulid | Identity of this grant or queue ticket. |
| `holder` | string? | Current holder's `agent_id` when not `granted`. |
| `lease_expires_at` | ts? | When a `granted` hold auto-expires. |
| `queue_position` | int? | Position when `queued`. |
| `fence` | int | Monotonic fence token; rises on every grant of `resource`. |

#### `release` — relinquish a held claim (or cancel a queued ticket)

Request:
| field | type | meaning |
|---|---|---|
| `agent_id` | string | Caller (must be the holder/ticket owner). |
| `claim_id` | ulid | The grant or ticket to release. |
| `resource` | string | The resource (cross-checked against `claim_id`). |

Response:
| field | type | meaning |
|---|---|---|
| `status` | enum | `released` \| `not_holder` \| `unknown`. |
| `next_holder` | string? | Agent promoted from the queue, if any. |
| `fence` | int? | New fence after promotion. |

#### `claims` — inspect current locks and queues

Request:
| field | type | meaning |
|---|---|---|
| `agent_id` | string | Caller. |
| `resource` | string? | Filter to one resource; omit for all. |

Response:
| field | type | meaning |
|---|---|---|
| `claims` | list | One entry per live resource. |
| `claims[].resource` | string | Lock key. |
| `claims[].mode` | enum | `exclusive` \| `shared`. |
| `claims[].holder` | string \| list | Holder (or holders for `shared`). |
| `claims[].claim_id` | ulid | Active grant id. |
| `claims[].fence` | int | Current fence token. |
| `claims[].lease_expires_at` | ts | Hold auto-expiry. |
| `claims[].queue` | list | Waiting tickets (`agent_id`, `claim_id`, position). |
| `claims[].note` | string? | Holder's stated reason. |

### 4.3 Family: messaging — `post`, `inbox`, `read`

Durable mail that survives sender death.

#### `post` — send a durable message

Request:
| field | type | meaning |
|---|---|---|
| `agent_id` | string | Sender. |
| `to` | string | Recipient: an `agent_id`, `*` (broadcast), or `topic:<name>`. |
| `subject` | string? | Optional short subject. |
| `body` | string | Message content (text). |
| `reply_to` | ulid? | Threads this as a reply to a prior message. |
| `ttl_seconds` | int? | Optional auto-expiry; omit for durable-until-GC. |

Response:
| field | type | meaning |
|---|---|---|
| `message_id` | ulid | Sortable, time-ordered id (also the cursor unit). |
| `posted_at` | ts | Server time. |
| `fanout` | int | Number of recipients the message was addressed to. |

#### `inbox` — peek pending messages without advancing the cursor

Request:
| field | type | meaning |
|---|---|---|
| `agent_id` | string | Caller. |
| `since` | ulid? | Return messages after this id; default = caller's cursor. |
| `topics` | list? | Additional `topic:` subscriptions to include. |
| `limit` | int? | Max messages returned. |

Response:
| field | type | meaning |
|---|---|---|
| `messages` | list | Pending messages, time-ordered. |
| `messages[].message_id` | ulid | Id / cursor unit. |
| `messages[].from` | string | Sender. |
| `messages[].to` | string | Original addressing (`agent_id`/`*`/`topic:`). |
| `messages[].subject` | string? | Subject. |
| `messages[].body` | string | Content. |
| `messages[].posted_at` | ts | Server time. |
| `messages[].reply_to` | ulid? | Thread parent. |
| `cursor` | ulid | Caller's current read cursor (unchanged by `inbox`). |
| `unread` | int | Count pending beyond the returned page. |

#### `read` — advance the read cursor (acknowledge consumption)

Request:
| field | type | meaning |
|---|---|---|
| `agent_id` | string | Caller. |
| `up_to` | ulid | Mark everything through this `message_id` as read. |

Response:
| field | type | meaning |
|---|---|---|
| `cursor` | ulid | New cursor position. |
| `remaining` | int | Messages still pending after the advance. |

### EARS — verb contract

- **V-1.** The system **shall** expose exactly eight MCP verbs:
  `register`, `roster`, `claim`, `release`, `claims`, `post`, `inbox`, `read`.
- **V-2.** Each verb **shall** require the caller's `agent_id` and **shall** treat
  it as the authenticated principal for the trust rules in §8.
- **V-3.** The system **shall** return structured responses matching the shapes in
  §4.1–§4.3 for every verb.
- **V-4.** The API surface **shall not** expose, require, or assume any particular
  delivery model (poll vs hub-relay); delivery is a consumption concern handled
  entirely through `inbox`/`read` and is independent of how messages are stored
  (see §6, §7).

---

## 5. Atomic-claim semantics

The claim lifecycle is **grant → hold → (queue) → release**, with staleness and
death handling layered on lease expiry.

### 5.1 Grant

- **C-1.** **When** an `exclusive` claim is requested for a resource with no live
  holder, the system **shall** grant it within a single atomic write transaction
  and return `granted` with a fresh `claim_id` and an incremented `fence`.
- **C-2.** **When** a `shared` claim is requested and the resource has only other
  `shared` holders (or none), the system **shall** add the caller as a co-holder.
- **C-3.** **If** two agents request the same `exclusive` resource concurrently,
  **then** the system **shall** grant exactly one and the other **shall** receive
  `denied` or `queued` (never both granted). *(Why: this is the cross-process CAS
  git cannot provide; correctness here is the daemon's reason to exist.)*
- **C-4.** Each grant of a resource **shall** carry a monotonically increasing
  `fence` token so a holder can prove its grant is current and a stale holder's
  late action can be detected and ignored by convention.

### 5.2 Hold

- **C-5.** A granted claim **shall** be held until released, lease expiry, or
  holder death — whichever comes first.
- **C-6.** **While** an agent holds a claim, that hold **shall** be visible in
  `claims` and in the holder's `roster` `held_claims`.

### 5.3 Queue

- **C-7.** **When** an `exclusive` resource is held and the caller requested
  `wait: queue`, the system **shall** enqueue a ticket and return `queued` with a
  `queue_position`.
- **C-8.** **When** a held claim is released or expires, the system **shall**
  promote the oldest waiting ticket to holder atomically, increment `fence`, and
  record the promotion (surfaced as `next_holder` to the releaser).
- **C-9.** **If** the caller requested `wait: no_wait` on a held resource,
  **then** the system **shall** return `denied` immediately and enqueue nothing.

### 5.4 Release, staleness, and death (expiry)

- **C-10.** **When** a holder calls `release` with a matching `claim_id` and
  `resource`, the system **shall** free the hold and promote the queue per C-8.
- **C-11.** **If** `release` is called by an agent that is not the holder/ticket
  owner, **then** the system **shall** reject it as `not_holder` and leave the
  hold unchanged. *(See trust §8.)*
- **C-12.** **When** a claim's `lease_expires_at` passes without a renewal, the
  system **shall** treat the claim as expired, free it, and promote the queue —
  so a crashed agent cannot hold a lock forever. *(Why: agents die; locks must
  self-heal. This is the death/expiry safety valve.)*
- **C-13.** **When** an agent's roster liveness transitions to `dead`
  (§6), the system **shall** release all claims that agent holds and cancel its
  queued tickets, promoting waiters accordingly.
- **C-14.** **Where** a holder wishes to keep a long-running lock, the system
  **shall** accept lease renewal (re-issuing `claim` for the same resource by the
  current holder extends `lease_expires_at` without changing `fence`).
- **C-15.** A `claim` request **shall** be idempotent for the current holder: a
  repeat by the holder returns the existing grant (renewed per C-14), not a
  conflict.

---

## 6. Roster and liveness model

- **R-1.** The system **shall** maintain one roster entry per `agent_id`, created
  or refreshed by `register`.
- **R-2.** Each roster entry **shall** carry `last_seen`, `expires_at`
  (`last_seen + ttl`), `branch`, `prompt_ptr`, optional `role`, and an `epoch`.
- **R-3.** The system **shall** derive liveness as: `alive` while
  `now <= expires_at`; `stale` for a bounded grace window after `expires_at`;
  `dead` once the grace window elapses.
- **R-4.** **When** an entry becomes `dead`, the system **shall** trigger claim
  cleanup per C-13 and **shall** retain the entry as `dead` in `roster` only when
  `include_stale` is requested, so peers can observe a recent departure.
- **R-5.** **When** an agent re-registers after being `dead`, the system **shall**
  increment its `epoch` so peers can detect the restart.
- **R-6.** Liveness **shall** be derived from heartbeat freshness only; the system
  **shall not** infer liveness from git commit timestamps (that is git-fs's
  domain, and a quiet agent is still alive).

---

## 7. Delivery-constraint resolution (binding)

### 7.1 The hard harness constraint

A dispatched subagent **cannot receive an unsolicited push mid-turn**. The
harness primitives that would interrupt an agent (`ScheduleWakeup`,
`SendMessage`) are **driver-only** — a subagent inspecting its own tool registry
will not find them. Therefore `mesh`'s "notify" is **not a true interrupt**. A
posted message cannot reach into a running agent's turn.

This SPEC resolves delivery with two models, and requires the storage model to be
ignorant of both (V-4).

### 7.2 Primary model — cooperative polling

- **D-1.** The PRIMARY delivery model **shall** be cooperative polling: agents
  call `inbox` (and `claims`) at their own **step checkpoints** — natural
  boundaries between units of work — and act on what they find.
- **D-2.** The system **shall** make polling cheap and correct: `inbox` returns
  only messages after the caller's cursor, and `read` advances the cursor, so an
  agent that polls every checkpoint sees each message exactly once.
- **D-3.** Because polling is cooperative, the message store **shall** be durable
  independent of whether or when any recipient polls — an unpolled message waits
  in the store until read or GC'd (see durability, §9).

### 7.3 Alternative model — hub-relay via the orchestrator

- **D-4.** **Where** lower latency than checkpoint polling is required, the system
  **shall** support a hub-relay model in which the **orchestrator driver** (which
  *does* hold `SendMessage`/`ScheduleWakeup`) polls `mesh` on the fleet's behalf
  and wakes a target agent via `SendMessage` when a message or grant is pending.
- **D-5.** The hub-relay model **shall** use the *same* `inbox`/`read` verbs and
  cursors as the polling model; the relay is just another reader acting on behalf
  of an agent. No new storage, addressing, or verb shall be introduced for it.
- **D-6.** The system **shall not** assume either model in its storage: messages,
  cursors, and claims are stored identically regardless of who polls or whether a
  relay is in use. *(Why: keeps delivery a pure consumption concern; either model
  can be adopted or mixed per deployment without a data migration.)*

---

## 8. Security / trust model

Trust is per-`agent_id`. The fleet is cooperative but the daemon enforces
ownership so a buggy or confused agent cannot corrupt others' coordination state.

- **T-1.** The system **shall** treat the request's `agent_id` as the principal
  and authorize every mutation against it.
- **T-2.** **Only** the holder (or ticket owner) of a claim **shall** release or
  renew it; a `release`/renew by any other `agent_id` **shall** be rejected
  (`not_holder`) — except the daemon's own automatic expiry/death cleanup
  (C-12/C-13), which is system-initiated, not agent-initiated.
- **T-3.** An agent **shall** read only its own inbox and the topics/broadcasts it
  is addressed by; one agent **shall not** read or advance another agent's read
  cursor. *(Mail is private to its recipient; broadcast and topic messages are
  readable by any subscriber by definition of their addressing.)*
- **T-4.** Any agent **may** `post` to any `agent_id`, to `*`, or to a `topic:`;
  posting is open because messages are additive and cannot corrupt a recipient's
  state (the recipient still controls its own cursor).
- **T-5.** Any agent **may** call read-only awareness/inspection verbs (`roster`,
  `claims`, `inbox` for itself); these expose coordination state, not code or
  memory.
- **T-6.** The daemon **shall** serve a single repo working directory (per-cwd)
  and **shall not** accept or relay coordination state across unrelated cwds;
  cross-fleet federation is out of scope (§ Exclusions). *(Why: scope of trust is
  one repo's fleet, matching the per-cwd daemon boundary kern uses.)*
- **T-7.** The system **shall not** authenticate beyond `agent_id` identity
  asserted by the local MCP client; it assumes a single trusted local fleet, not
  a hostile multi-tenant environment. This assumption is stated so it is not
  mistaken for a security guarantee.

---

## 9. Message model — addressing, topics, broadcast, durability, cursors

- **M-1.** A message **shall** be addressed to exactly one of: a specific
  `agent_id` (point-to-point), `*` (broadcast to the whole fleet), or
  `topic:<name>` (all current subscribers of a topic).
- **M-2.** Topic subscription **shall** be expressed by the reader at poll time
  (`inbox.topics`) rather than by a registration step, so a late-joining agent can
  read a topic's pending messages from its own cursor without having pre-declared
  interest. *(Why: durability over presence — see M-4.)*
- **M-3.** Each message **shall** receive a sortable, time-ordered `message_id`
  that doubles as the cursor unit, giving a total order per recipient.
- **M-4.** A message **shall** be durable: it **shall** survive the death of its
  sender and **shall** be deliverable to a recipient that was not alive when it
  was posted, until the recipient reads past it or it is GC'd/expired.
- **M-5.** The system **shall** maintain a per-`agent_id` read cursor in the
  journal so that `inbox` returns each message at most once per recipient and the
  cursor survives daemon restart.
- **M-6.** **Where** a message carries `ttl_seconds`, the system **shall** expire
  it after the TTL even if unread; otherwise a message persists until read by all
  its addressees (or, for `*`/`topic:`, until GC reclaims aged-out messages).
- **M-7.** The system **shall** record threads via `reply_to` so a reply links to
  its parent `message_id`, without requiring a separate thread object.
- **M-8.** Broadcast and topic messages **shall** be stored once and read through
  each recipient's own cursor — the store **shall not** duplicate the body per
  recipient. *(Why: a fleet-wide broadcast must not multiply storage by fleet
  size.)*

---

## Exclusions (What NOT to Build)

- **X-1. No code, working-tree, or change-history ownership.** That is git-fs's
  job; `mesh` stores no file contents and no commits.
- **X-2. No declared-intent storage.** The roster holds a *pointer* to the
  git-fs-owned `.machine-prompt`, never the intent text.
- **X-3. No memory/knowledge-graph features.** Recall of *why* decisions were made
  stays in `kern`; `mesh` does not ingest, embed, or reason over content.
- **X-4. No true mid-turn interrupt / push delivery.** The harness forbids it for
  subagents (§7.1). `mesh` provides durable mail + cooperative poll + optional
  hub-relay, not an OS-style signal.
- **X-5. No cross-cwd / cross-repo federation or gossip of coordination state.**
  `mesh` is per-cwd; one repo's fleet only. (kern's gossip `peers` model is *not*
  copied for coordination state.)
- **X-6. No human chat UI / presence dashboard.** `mesh` is an agent-facing MCP
  surface; any human-facing view is a separate concern built on top of `roster`.
- **X-7. No message-ordering guarantees across recipients.** Order is total *per
  recipient cursor*, not a global fleet clock.
- **X-8. No authn beyond local `agent_id` trust** (§T-7). Not a multi-tenant
  security boundary.
- **X-9. No conflict resolution of code edits.** Claims advise who *should* edit
  what; the actual merge/conflict story remains git-fs's (commits + Stop-hook
  merge).

---

## Acceptance criteria

1. **Two-layer boundary holds.** Given an agent posts a message and acquires a
   claim, when the repo's git history is inspected, then no roster entry, claim
   record, or message body appears in any commit, and the only intent reference in
   `mesh` is a pointer to the git-fs `.machine-prompt` (A-1, A-2, A-3, X-1, X-2).
2. **Sibling, not overlap.** Given both daemons run, when each writes its state,
   then `mesh` and `kern` use separate per-cwd data directories and neither writes
   into the other's store; `mesh` exposes no memory/recall verbs (A-4, X-3).
3. **Exactly eight verbs, contract-shaped.** Given the MCP server is attached,
   when its tool list is read, then it advertises `register`, `roster`, `claim`,
   `release`, `claims`, `post`, `inbox`, `read` and nothing else, and every
   response matches §4 shapes (V-1, V-3).
4. **Atomic exclusive grant under contention.** Given two agents request the same
   `exclusive` resource concurrently, when both calls return, then exactly one is
   `granted` and the other is `denied` or `queued` — never two grants — and the
   grant's `fence` is strictly greater than any prior grant of that resource
   (C-1, C-3, C-4, S-2).
5. **Queue promotion on release.** Given resource R is held and a second agent is
   `queued`, when the holder calls `release`, then the queued agent becomes holder
   atomically, `fence` increments, and the releaser sees it as `next_holder`
   (C-7, C-8, C-10).
6. **Lease expiry self-heals a dead holder's lock.** Given an agent holds R and
   then crashes without releasing, when its `lease_expires_at` passes, then R is
   freed and any waiter is promoted — no lock is held forever (C-12, C-13).
7. **Non-holder cannot release.** Given agent A holds R, when agent B calls
   `release` on R, then the call is rejected `not_holder` and A still holds R
   (C-11, T-2).
8. **Durable mail survives sender death.** Given agent A posts to agent B and then
   A dies, when B (which may not have existed at post time) later calls `inbox`,
   then B receives A's message exactly once and `read` advances B's cursor so a
   re-poll does not re-deliver it (M-4, M-5, D-2).
9. **Broadcast stored once, read per cursor.** Given an agent posts to `*` in a
   fleet of N, when each recipient polls, then each sees the message exactly once
   through its own cursor and the store holds a single copy of the body (M-1, M-8).
10. **Topic readable by a late joiner.** Given a message was posted to
    `topic:build` before agent C registered, when C calls `inbox` with
    `topics:[build]`, then C receives the pending topic message from its own
    cursor without having pre-subscribed (M-2, M-4).
11. **Liveness from heartbeat, not git.** Given an agent stops heartbeating while
    its branch still has recent commits, when `roster` is queried after the TTL +
    grace window, then the agent reads as `dead` and its claims are released
    (R-3, R-4, R-6, C-13).
12. **Privacy of mail and cursors.** Given agent A's inbox, when agent B calls any
    verb, then B cannot read A's point-to-point messages nor advance A's cursor;
    B can read only `*`/`topic:` messages it is addressed by (T-3).
13. **Delivery model is storage-agnostic.** Given the cooperative-poll model in
    use, when the deployment switches to hub-relay (orchestrator polls and wakes
    via `SendMessage`), then no message, cursor, or claim data migration is
    required and the same `inbox`/`read` verbs serve both (D-4, D-5, D-6, V-4).
14. **Storage decision honored.** Given the daemon's data directory, when its
    files are inspected, then claims/roster/message records live in an LMDB
    primary store and an ordered message/claim/cursor log lives in a SQLite-WAL
    journal, and the data directory is gitignored (S-1, S-3, S-4).
15. **Matches kern's lifecycle.** Given the machine plugin is installed, when the
    fleet starts, then `mesh` launches as a per-cwd native-binary MCP server over
    stdio (the same idiom as `kern mcp`) and is reachable as `mcp__mesh__*`
    (L-1, L-2).

---

## Quality gate / Definition of Done (this SPEC)

This SPEC is "done" when: it is uniquely IDed (`SPEC-COMM-001`), is EARS-compliant
throughout, contains the two-layer architecture, the full eight-verb contract with
request+response shapes, atomic-claim semantics (grant/hold/queue/release +
staleness/expiry/death), the message model (addressing/topics/broadcast/durability
/cursors), the delivery-constraint resolution (poll primary + hub-relay alt), the
storage + language decision with rationale, the roster/liveness model, the trust
notes, a non-empty Exclusions section, and a numbered acceptance-criteria list — and
contains no implementation detail (function names, struct layouts, wire formats)
beyond the contract shapes required to define behavior.
