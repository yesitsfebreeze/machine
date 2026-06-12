# profiling

**When**: investigating slowdowns, validating an optimization, picking a profiler.
**Why care**: optimizing without measuring is guessing; the wrong tool hides the real bottleneck.

## Decision tree
- "Where does time go?" → sampling profiler. Reason: low overhead, captures real distribution.
- "Why is this function slow?" → instrumentation / tracing on the path. Reason: per-call detail.
- Concurrency stalls / lock waits → off-CPU profiling. Reason: sampling misses sleeping threads.
- Memory regression → heap profiler / allocation tracker. Reason: CPU profilers miss allocations.

## Tradeoffs
- Sampling: low overhead, statistical (misses rare events).
- Instrumentation: precise, perturbs the program.
- Tracing: full timeline, expensive to store/analyze.
- Microbenchmarks: control variables, often unrepresentative of real workloads.

## Anti-patterns (why)
- Optimizing without a baseline: can't prove improvement, can regress.
- Profiling debug builds: noise dominates; conclusions misleading.
- Trusting a single run: variance can swamp a 20% delta.
