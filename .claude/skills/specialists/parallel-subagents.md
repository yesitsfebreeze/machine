# parallel-subagents

**When**: deciding to fan out work, isolating context, splitting research from synthesis.
**Why care**: serial work wastes wallclock; unprincipled fan-out wastes tokens and produces incoherent merges.

## Decision tree
- 2+ independent units of work → parallel. Reason: wallclock + context isolation.
- One agent's output feeds another's input → sequential. Reason: parallel would race or duplicate.
- Output would flood main context → delegate to subagent, take summary. Reason: main thread stays lean.
- Risk of divergent decisions → keep in one agent. Reason: parallel agents can't coordinate mid-flight.

## Tradeoffs
- Fan-out: faster wallclock, harder to merge findings.
- Sequential: coherent, slower.
- Subagent for context shielding: keeps main thread lean, pays a roundtrip.

## Anti-patterns (why)
- Splitting one logical task across agents: each sees half the picture, both make local-optimum choices.
- Spawning agents for trivial single-file edits: orchestration overhead exceeds the work.
- Letting parent re-do the search the subagent already did: doubles cost.
