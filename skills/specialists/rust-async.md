# rust-async

**When**: choosing runtime, deciding sync vs async, handling blocking calls, `Send` bounds.
**Why care**: async wrong-fits CPU work; blocking inside async stalls the executor; runtime mixing causes lockups.

## Decision tree
- IO-bound, many concurrent tasks → async (tokio default in ecosystem). Reason: cheap task switches beat OS threads at scale.
- CPU-bound → threads (rayon for data-parallel). Reason: async gives no speedup for compute; only adds polling cost.
- Blocking call inside async context → `spawn_blocking`. Reason: blocks executor thread otherwise, starves other tasks.
- Library code → stay runtime-agnostic if possible. Reason: locks consumers into one runtime.
- Need `Send` future → audit captured types. Reason: non-Send types (Rc, RefCell) prevent cross-thread scheduling.

## Tradeoffs
- tokio: ecosystem default, multi-threaded. Pay: complexity, `Send` bounds everywhere.
- smol/async-std: simpler. Pay: smaller ecosystem.
- Single-threaded runtime: removes `Send` bounds. Pay: no parallelism.
- "async all the way down" is dogma; sync boundaries are fine when the work is short and bounded.

## Anti-patterns (why)
- `block_on` inside async: deadlock risk on single-threaded runtimes.
- Holding sync `Mutex` across `.await`: blocks the executor; use async mutex or restructure.
- Spawning futures without bounding: unbounded concurrency → resource exhaustion.
