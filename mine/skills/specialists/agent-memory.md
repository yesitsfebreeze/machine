# agent-memory

**When**: deciding what to persist across sessions, designing retrieval, choosing memory shape.
**Why care**: memory bloat poisons future context; missing memory makes the agent stateless and frustrating.

## Decision tree
- Fact about the user/project that future sessions need → persist. Reason: avoids re-asking.
- Solution to a specific bug → don't persist; the code/commit holds it. Reason: code is authoritative.
- Surprising preference or constraint → persist with reason. Reason: future sessions can apply nuanced judgment.
- Snapshot of state (file list, activity) → persist sparingly; verify before use. Reason: stale fast.

## Memory types (why each)
- Episodic: what happened, when. Useful for "what did we try last time?"
- Semantic: facts about user/project. Useful for tailoring style.
- Procedural: how-to recipes. Useful when the user repeats a workflow.

## Retrieval triggers
- User references past work → query memory.
- Behavioral drift risk → load feedback memories at session start.
- Conflict with current state → trust current, update memory.

## Anti-patterns (why)
- Saving everything: high recall, low precision; context drowns in noise.
- Saving fix recipes that the code already encodes: duplication that rots.
- Index files growing without bound: load cost dominates value.
