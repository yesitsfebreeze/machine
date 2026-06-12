---
name: specialists
description: Sub-loader for domain specialists (Rust, agent infra, terminal, harness, perf, security, speed). Each entry captures the WHY and decision tree only — not implementation details. Trigger via `/specialists` or whenever a decision in one of these domains is on the table.
---

# Specialists

Index of domain specialists. Each file answers **why** to pick an approach and **when** — never **how**. Implementation details (APIs, constants, syntax) stay in code and library docs (context7).

## When to consult
- Before a non-trivial design decision in the listed domain.
- During review when a choice feels arbitrary — load the specialist to check the why.
- When unsure which approach fits — read the decision tree first, then commit.

## Index

### Rust systems
| Specialist | When to load |
|------------|--------------|
| [rust-ownership](rust-ownership.md) | Choosing borrow vs clone vs Arc/Rc/Cow; refactor away from `Arc<Mutex<T>>` smell |
| [rust-async](rust-async.md) | Picking a runtime, sync↔async boundary, `Send` bound problems, blocking inside async |
| [rust-errors](rust-errors.md) | Designing error types, library vs app style, where to `?` vs handle |
| [rust-traits](rust-traits.md) | Static vs dynamic dispatch, type-state, sealed traits, generic-vs-`dyn` |

### Agent / AI infra
| Specialist | When to load |
|------------|--------------|
| [mcp-design](mcp-design.md) | Building MCP servers, tool granularity, MCP vs in-process |
| [prompt-caching](prompt-caching.md) | Structuring prompts for reuse, cache breakpoints, invalidation reasons |
| [agent-memory](agent-memory.md) | What to persist across sessions, retrieval triggers, memory shape |
| [tool-routing](tool-routing.md) | Eager vs deferred tool loading, large catalogs, dispatch accuracy |

### Terminal / TUI (no Tauri)
| Specialist | When to load |
|------------|--------------|
| [ratatui-arch](ratatui-arch.md) | Structuring a ratatui app, state ownership, blocking work |
| [ansi-pty](ansi-pty.md) | Escape sequences, capability detection, PTY semantics |
| [terminal-input](terminal-input.md) | Kitty protocol, mouse, bracketed paste, raw mode safety |

### Agent harness
| Specialist | When to load |
|------------|--------------|
| [agent-sandbox](agent-sandbox.md) | Running model-generated commands, fs/net scoping, blast radius |
| [harness-hooks](harness-hooks.md) | Pre/Post tool hooks, SessionStart, enforcing invariants |
| [parallel-subagents](parallel-subagents.md) | Fan-out decisions, context isolation, when sequential wins |
| [context-mgmt](context-mgmt.md) | Compaction, deferred tools, summarization triggers |

### Performance
| Specialist | When to load |
|------------|--------------|
| [profiling](profiling.md) | Picking a profiler, baselines, measurement discipline |
| [cache-locality](cache-locality.md) | Data layout in hot loops, SoA/AoS, false sharing |
| [allocator](allocator.md) | Arena/slab/system choice, lifetime-based reasoning |
| [contention](contention.md) | Lock-free vs mutex vs sharding, RwLock pitfalls |

### Security
| Specialist | When to load |
|------------|--------------|
| [supply-chain](supply-chain.md) | New deps, lockfile policy, audit cadence |
| [secrets](secrets.md) | Storing credentials, rotation, blast radius |
| [capability-sandbox](capability-sandbox.md) | Permission model for tools/plugins/agents |
| [input-validation](input-validation.md) | Trust boundaries, parse-don't-validate |

### Speed (latency-first vs perf throughput-first)
| Specialist | When to load |
|------------|--------------|
| [startup-latency](startup-latency.md) | CLI cold start, lazy init, daemonization |
| [tail-latency](tail-latency.md) | p99/p999 SLOs, fan-out, hedging |
| [streaming-batch](streaming-batch.md) | Stream vs micro-batch vs full batch, backpressure |

## Format contract
Every specialist file follows:
- **When** (trigger)
- **Why care** (consequence of getting it wrong)
- **Decision tree** (if X → Y, *because reason*)
- **Tradeoffs** (A vs B, what each pays)
- **Anti-patterns** (why each is bad, not how to write the good version)

No code, no constants, no syntax. If a file accumulates implementation details, refactor — that content belongs in the relevant library doc (use context7) or the source itself.

## Maintenance
- Each specialist has a corresponding vicky pending research entry for future enrichment.
- When the user gives feedback that contradicts a specialist, update the file (not memory).
- Adding a new specialist: drop a file here, add a table row, add to vicky queue.
