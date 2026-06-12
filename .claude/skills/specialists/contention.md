# contention

**When**: multithread bottleneck, deciding lock-free vs mutex vs sharding.
**Why care**: contention turns parallel work serial; lock-free is hard to get right and rarely needed.

## Decision tree
- Critical section is tiny + uncontended → mutex. Reason: simple, fast in fast path.
- Read-heavy, rare writers → RwLock or RCU. Reason: parallel readers.
- High contention, simple op → atomic / lock-free. Reason: avoid OS wait queue.
- Workload partitionable → shard by key. Reason: removes contention entirely.
- Producer/consumer → channel. Reason: ownership-passing eliminates shared mutation.

## Tradeoffs
- Mutex: easy, predictable, can starve under contention.
- RwLock: scales reads, writer starvation risk; reader/writer cost asymmetric.
- Lock-free: scales, very hard to write correctly; ABA, memory ordering pitfalls.
- Sharding: best when keys distribute uniformly; hot keys defeat it.

## Anti-patterns (why)
- Lock-free as default: complexity tax, often slower under low contention.
- Coarse global lock around fast work: serializes everything for one slow op.
- Holding a lock across IO: turns lock contention into IO contention.
