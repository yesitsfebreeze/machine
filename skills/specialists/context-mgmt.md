# context-mgmt

**When**: long sessions, large tool outputs, choosing summarization vs deferral vs delegation.
**Why care**: context is finite and expensive; cache invalidation by reordering wastes spend; loss of key facts breaks coherence.

## Decision tree
- Tool output >20 lines → sandbox-and-summarize. Reason: raw flood eats budget for no recall benefit.
- Long-running session approaching budget → compact prior turns. Reason: continued work needs headroom.
- Repeated questions about codebase → graph/index once, query many. Reason: re-grepping is wasteful.
- Tool unlikely to be reused → defer schema loading. Reason: schemas are permanent context cost.

## Tradeoffs
- Eager summarization: lean context, risk dropping a detail later needed.
- Lazy / on-demand recall: full fidelity, pay each retrieval.
- Subagent offload: best isolation, pay coordination overhead.

## Anti-patterns (why)
- Reading 2000-line files just to find one function: use grep/glob first.
- Re-pasting prior content into a new turn: invalidates cache from that point.
- Keeping ephemeral planning docs in-conversation forever: nothing is reusing them.
