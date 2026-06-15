# Principles — KISS, DRY, YAGNI

Three rules for keeping code small and changeable.

## KISS — Keep It Simple

Simplest design that works. Short functions, shallow nesting, plain control flow.

## DRY — Don't Repeat Yourself

> Every piece of *knowledge* must have a single, unambiguous, authoritative representation. — Hunt & Thomas

DRY = knowledge, not character duplication. Same shape + different reasons-to-change ≠ violation.

## YAGNI — You Aren't Gonna Need It

Don't build until required. Four costs of speculative features (Fowler 2015):
1. **Build** — time on unused capability
2. **Delay** — work pushed back
3. **Carry** — code mass slowing every future change
4. **Repair** — fix when wrong (⅔ of speculative features are — Kohavi)

## Rules of thumb

- **Rule of three.** First: write. Second: notice. Third: extract.
- **Imagine the refactor.** Picture the diff to introduce later. Usually no worse than now.
- **Early returns over nested else.** Guard clauses flatten.
- **Composition > inheritance.** Inheritance couples; composition combines.
- **Validate at boundaries, trust internals.** Edges = user input + external APIs.
- **Three similar lines beat premature abstraction.** Wait for third.
- **No half-finished implementations.** Ship slice or don't start.

## Don't add

- Error handling for impossible cases
- Fallbacks outside system boundaries
- Validation of internal callers
- Feature flags / backwards-compat shims (just change the code)
- Comments explaining *what* well-named code shows
- References to current task/PR/issue (rot)

## Add

- Comment only to prevent a real head-scratcher, or as a utility marker: hidden constraint, subtle invariant, workaround for a specific bug, surprising behavior. If well-named code already shows it, don't.

If removing a comment wouldn't confuse reader, don't write it.
