# cache-locality

**When**: data structure layout decisions in hot loops, sharing across cores.
**Why care**: cache misses dominate runtime in tight loops; false sharing destroys multicore scaling.

## Decision tree
- Iterating over one field of many records → Structure-of-Arrays. Reason: only the touched field streams through cache.
- Accessing all fields of one record → Array-of-Structures. Reason: spatial locality on the record.
- Two threads write to neighboring fields → pad to cache line. Reason: false sharing causes invalidation storms.
- Large struct in a tight loop → split hot/cold fields. Reason: cold fields evict hot ones.

## Tradeoffs
- SoA: better streaming, harder to reason about as a "thing".
- AoS: natural OO model, worse for column-wise scans.
- Padding: cheap memory, big perf win where false sharing exists.

## Anti-patterns (why)
- Cache-friendly micro-optimizations applied where the bottleneck is elsewhere: noise.
- Padding everything "just in case": balloons memory, evicts unrelated data.
- Assuming `Vec<Box<T>>` is contiguous: it's pointers to scattered boxes.
