# prompt-caching

**When**: structuring prompts/messages for repeated calls, deciding cache breakpoints.
**Why care**: cache misses cost full input tokens + latency; bad breakpoints invalidate stable prefixes.

## Decision tree
- Content stable across calls → place earliest, mark cacheable. Reason: prefix caching only works left-to-right.
- Content changes per call → place latest. Reason: keeps the long stable prefix reusable.
- System prompt + tool defs + memory + history → order most-stable-first. Reason: any change invalidates everything after.
- Cache TTL near expiry → consider refresh-on-write; otherwise next call eats full miss.
- Many small variants → cache the shared base, vary the tail.

## Tradeoffs
- Aggressive cache breakpoints: more reuse, more bookkeeping.
- Cache-friendly structure constrains prompt design (can't sprinkle dynamic data through stable sections).
- Provider-side TTLs (5 min on Anthropic) make sleep-between-calls expensive — see speed-vs-cache windowing.

## Anti-patterns (why)
- Timestamp in the system prompt: invalidates cache every call.
- Per-call random IDs in stable prefix: same.
- Re-ordering messages between calls: invalidates from the divergence point onward.
