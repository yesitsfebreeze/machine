# tool-routing

**When**: harness has many tools, deciding eager vs deferred loading, choosing dispatch path.
**Why care**: loaded-tool schemas eat context permanently; bad routing makes the model pick the wrong tool.

## Decision tree
- Tool used every session → eager-load. Reason: dispatch latency matters more than schema cost.
- Tool used occasionally → defer behind search/lookup. Reason: keeps idle context lean.
- Catalog > ~30 tools → search-based routing. Reason: model accuracy degrades with too many similar choices.
- Tool overlaps with another → merge or namespace. Reason: ambiguity = bad dispatch.

## Tradeoffs
- Eager loading: instant dispatch, fat context.
- Deferred + search: lean context, one extra hop per first use.
- Hierarchical (categories → tools): scales further; pays a category-pick step.

## Anti-patterns (why)
- Every MCP server eager-loaded: schemas can dominate context budget.
- Two tools that do nearly the same thing: model alternates randomly, results inconsistent.
- Tool names that don't describe action (`process`, `handle`): selection accuracy tanks.
