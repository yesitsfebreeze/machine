# Phase 6 — Adversarial pre-commit review

Final quality gate before commit.

## Self-review checklist

- [ ] All acceptance criteria addressed
- [ ] No hard-coded values that should be constants
- [ ] No assumptions made without verification
- [ ] All edge cases handled
- [ ] Error handling complete (only at boundaries — KISS)
- [ ] No security vulns (injection, XSS, SQLi, OWASP top 10)
- [ ] Tests cover new functionality
- [ ] Test suite passes
- [ ] Docs updated
- [ ] Follows existing patterns
- [ ] No half-finished implementations
- [ ] No speculative abstractions or feature flags
- [ ] No comments narrating *what* (only *why* when non-obvious)

## Final adversarial questions

- Runs twice concurrently?
- Input null/empty/negative/huge?
- Race conditions checked?
- Wrong assumptions?
- If breaking, how?
- Embarrassed if this broke in prod?

## TOCTOU prevention

```
WRONG: read state → [gap, others mutate] → act on stale state
CORRECT: lock → read → act → unlock
```

Applies to any shared mutable state: DBs, files, caches, APIs.

## Transaction side-effects

Code throwing inside a transaction rolls back ALL changes. If error-state must persist (mark failed, audit record), it must run **outside** the transaction.

## Shared-state documentation

Before changing shared mutable state, document:
1. All actors/methods that can modify
2. All concurrent scenarios
3. Invariants that must ALWAYS hold
4. Locking/coordination strategy

## Implementation rules

- Use existing abstractions — don't reinvent
- Use existing constants/enums/config — never hard-code
- Never skip input validation at boundaries
- Use project's established patterns (logging, errors, state)
- Trust internal callers — don't re-validate inside

## Stop

All checks pass.

## Anti-pattern

Treating bugs as one-off symptoms. Every bug = symptom. Find disease. What systemic issue allowed it? Where else does pattern appear?
