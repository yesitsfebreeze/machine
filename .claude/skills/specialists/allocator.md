# allocator

**When**: many small allocations, latency-sensitive paths, choosing an allocator strategy.
**Why care**: malloc/free overhead and fragmentation dominate workloads with churn; global allocator contention bottlenecks multithread.

## Decision tree
- Many short-lived allocations sharing a lifetime → arena/bump. Reason: bulk free, no per-alloc bookkeeping.
- Many same-sized allocations → slab/pool. Reason: O(1) alloc, no fragmentation.
- Cross-thread heavy allocation → thread-cached allocator (jemalloc/mimalloc). Reason: per-thread caches avoid global lock.
- Mostly long-lived → system allocator is fine. Reason: extra machinery doesn't pay back.

## Tradeoffs
- Arena: blazing fast, can't free individual items.
- Slab: tight memory, fixed shape only.
- Custom allocator: precise control, ownership of bugs (UB if wrong).

## Anti-patterns (why)
- Arena per request *and* freeing items inside: defeats the arena.
- Switching global allocator for the perf myth without measuring: variance, sometimes regression.
- Allocating in a hot loop where reuse would work: GC-like churn even in non-GC languages.
