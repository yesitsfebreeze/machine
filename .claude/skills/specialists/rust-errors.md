# rust-errors

**When**: designing error types, choosing library vs application error style, deciding where to `?` vs handle.
**Why care**: leaky error types couple modules; over-typed errors crush ergonomics; over-erased errors lose recovery info.

## Decision tree
- Library boundary → typed errors (thiserror-style). Reason: callers need to match and recover.
- Application top level → erased + context (anyhow-style). Reason: only need to log/display.
- Recoverable vs not → typed variants where caller might branch; opaque where it's terminal.
- Adding context as it bubbles → wrap with cause chain. Reason: stack trace alone loses domain info.

## Tradeoffs
- thiserror: explicit, exhaustive matching. Pay: boilerplate, churn on new variants.
- anyhow: ergonomic, dynamic. Pay: callers can't match without downcasting.
- "Result of Result" or deeply nested error enums: usually means the type is doing two jobs — split.

## Anti-patterns (why)
- `unwrap`/`expect` in library code: removes caller's choice to recover.
- Stringly-typed errors (`Err("something failed")`): unmatched, untranslatable.
- Catch-all `From` impls: silent error coercion hides bugs.
