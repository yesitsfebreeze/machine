# mcp-design

**When**: building an MCP server, deciding tool granularity, choosing MCP vs in-process lib.
**Why care**: chatty tools burn context; coarse tools hide intent; wrong boundary forces re-architecture.

## Decision tree
- Capability needs cross-agent/cross-language reuse → MCP server. Reason: protocol portability.
- Single agent, single language, hot path → in-process lib. Reason: no IPC overhead, simpler ops.
- One logical operation → one tool. Reason: tools that "do many things" force callers to compose via parameters, hurting model accuracy.
- Tool output likely >20 lines → return sandbox handle or summary, not raw. Reason: floods context.
- State across calls → server holds it, tool returns IDs. Reason: model doesn't have to re-pass blobs.

## Tradeoffs
- Fine-grained tools: accurate dispatch, more roundtrips.
- Coarse tools: fewer roundtrips, weaker model selection.
- Deferred/searchable tools: scale catalog, pay one ToolSearch hop. Worth it past ~30 tools.

## Anti-patterns (why)
- Returning megabytes from a tool: blows context window for everyone downstream.
- Mirroring REST verbs as tool names (GET/POST): model picks worse than verbs like `query`, `update`.
- Optional params with hidden defaults: model can't tell what changed.
