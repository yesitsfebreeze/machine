# tail-latency

**When**: SLOs measured at p99/p999, interactive workloads, fan-out systems.
**Why care**: tails dominate user-perceived performance; in fan-out, the slowest replica drives the response.

## Decision tree
- p99 spike correlated with GC/allocator → pre-allocate or switch allocator. Reason: pause time is the cause.
- p99 spike correlated with locks → reduce critical section or shard. Reason: queueing under contention.
- Fan-out request → hedging / backup requests. Reason: cut tail at the cost of small extra load.
- Long tasks blocking short ones → priority queues / separate pools. Reason: head-of-line blocking is structural.

## Tradeoffs
- Hedging: lower tails, extra capacity cost.
- Larger pools: reduces queueing, more memory.
- Pre-allocation: smooth tails, more steady-state memory.

## Anti-patterns (why)
- Optimizing the mean: invisible to users whose pain is at the tail.
- Logging at p99 without context: knowing it's slow doesn't tell you why.
- Treating p50/p99 as the same problem: causes are usually different.
