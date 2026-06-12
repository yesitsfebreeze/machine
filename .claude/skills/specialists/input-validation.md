# input-validation

**When**: data crossing a trust boundary; parsing user/network/file input.
**Why care**: most security bugs are validation gaps at boundaries; over-validating inside trusted code wastes effort and obscures intent.

## Decision tree
- Data from outside the process → validate at the boundary, once. Reason: single chokepoint, easy to audit.
- Data already parsed into a typed struct → trust it. Reason: redundant checks rot.
- Parse-don't-validate: turn raw input into a type that can't be invalid. Reason: invalid states become unrepresentable.
- Range/length limits → enforce at parse time. Reason: downstream code can assume invariants.

## Tradeoffs
- Strict parsers: safer, may reject legitimate-but-weird input.
- Permissive parsers: ergonomic, leave invariants implicit downstream.
- Schema-driven: centralized, can lag the code that uses it.

## Anti-patterns (why)
- Sanitizing at the point of use instead of the boundary: every use site is a potential miss.
- String validation by regex for structured data: misses edge cases, creates a second parser.
- Trusting "internal" RPC inputs: internal threat model is real; treat as boundary.
