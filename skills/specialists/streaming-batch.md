# streaming-batch

**When**: choosing between streaming results, micro-batching, or full-batch.
**Why care**: streaming buys first-token / first-row latency at throughput cost; batching wins throughput but hides progress.

## Decision tree
- User waits on output → stream. Reason: perceived latency = time to first token.
- Background pipeline, throughput-bound → batch. Reason: amortize per-item overhead.
- Need both → micro-batch. Reason: balances first-byte vs amortization.
- Downstream is itself streaming → stream end-to-end. Reason: buffering adds latency without payoff.

## Tradeoffs
- Streaming: low latency, complex backpressure, partial-failure handling.
- Batching: simple, high throughput, bad UX for interactive paths.
- Micro-batching: tunable middle ground, hyperparameter to maintain.

## Anti-patterns (why)
- Buffering a stream "for efficiency" at the UI: kills the only reason to stream.
- Unbounded buffer behind a producer: OOMs on backpressure absence.
- Batch sizes hand-tuned per environment without measurement: drift silently when workload changes.
