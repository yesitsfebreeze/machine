# Phase 4 — TDD with deep modules

**Symptom prevented:** large diffs then "remember" to type-check or test. **Cause:** outrunning headlights. Feedback rate = speed limit.

## RED → GREEN → Refactor

Per behavior in PRD test plan:

### 4.1 RED — failing test first

Test for behavior that doesn't exist. Run — MUST fail. Test passing pre-impl tests nothing.

Test at **interface boundary** from PRD. Use canonical glossary terms in test names + assertions.

Naming: `process_should_return_error_when_input_empty()`.

### 4.2 GREEN — minimal impl

Min code to pass test. No gold-plating. No "while I'm here".

### 4.3 Refactor — deepen module

Hide complexity, simplify interface, reduce surface. After green, before next RED.

## Mutation testing mindset

- Don't just assert success — assert specific values, counts, state changes
- Boundary conditions: `> 0` → test 0, 1, -1
- Side effects: method updates multiple fields → assert ALL
- One assertion per test when possible
- If `>` → `>=` mutation, test catch it? If not, add one

## Feedback loop after each cycle

- Static types
- Automated tests
- Browser (frontend), real DB (integration) where applicable

UI/frontend: dev server + exercise feature in browser. Type checks + test suites verify code correctness, not feature correctness.

## Deep modules, not shallow (Ousterhout)

- **Deep:** lots of functionality, simple interface, complexity hidden
- **Shallow:** little functionality, complex interface, deps leak

Test at deep-module interfaces, not every shallow function.

If shallow-module sprawl in area you're touching, **stop and run phase 5 first** on the slice. Cannot extend shallow thicket correctly.

## Test strategy by complexity

| Change | Strategy |
|---|---|
| Single file fix <20 lines | Related test class only |
| Single file 20-50 lines | Related tests + sanity |
| Multi files, same feature | Feature suite |
| Cross-cutting | All affected modules |
| DB/schema | All affected modules |
| Auth/security | All affected modules |

## Commit cadence

After each green-refactor. See `commits.md`. Reference PRD ID + decision IDs in body when *why* not obvious.

If tests fail: analyze (don't guess), fix root cause (not symptom), re-run, repeat to 0.

**Never commit failing tests.**

## Stop

All PRD behaviors have passing tests at right boundary. Static types pass. No new shallow modules.

## Anti-pattern

Big-batch impl + "let me add tests now". = implementation-driven dev with test suffix.
