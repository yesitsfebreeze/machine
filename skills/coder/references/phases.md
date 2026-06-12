# Phases 0-3, 5

Phase 4 (TDD) → `tdd.md`. Phase 6 (adversarial) → `adversarial.md`. Phase 7 (push) → `push.md`.

Tool-agnostic. Use whatever persistence the project provides (wiki MCP, knids, ADRs, docs/, none).

---

## Phase 0 — Orient

**Goal:** establish what's known before drafting.

1. Identify persistence layer (knowledge store, `docs/`, ADRs, prior PRs, none).
2. Query 2-3 phrasings of intent. Read top hits in full.
3. Codebase search for prior implementations of same concept.
4. Identify load-bearing prior decisions.
5. Surface inline: "Project knows X. Top sources: A, B, C. Open: D, E."

**Stop:** can describe what's known + what isn't. User confirms framing.

If nothing on topic, say so. No fabrication.

---

## Phase 1 — Align (design concept)

**Symptom prevented:** AI builds wrong thing. **Cause:** no shared design concept (Brooks).

1. Draft candidate design concept: what/why/scope/non-goals/constraints. Cite sources per claim.
2. Walk design tree (Brooks): list major decisions + dependencies. Recommend each, citing prior decisions.
3. Surface: **"Is this the plan? Layout match your mental model?"**
4. Resolve disagreements one branch at a time. No moving on with unresolved branches.
5. If answerable from codebase, read it; don't ask.
6. On convergence: record design concept (commit note, knowledge store, issue).

**Stop:** user + AI describe same thing in different words, including boundaries + non-goals. Decision tree resolved. Recorded.

**Anti-pattern:** jumping to PRD before user confirms design concept.

---

## Phase 2 — Glossary (ubiquitous language)

**Symptom prevented:** verbose AI, term drift. **Cause:** no shared vocabulary (Evans, DDD).

1. Query for prior glossary.
2. Scan phase 0+1 for domain nouns/verbs/concepts.
3. Identify problems: same word/different concepts (flag), different words/same concept (canonicalize), vague/overloaded (sharpen).
4. Draft `UBIQUITOUS_LANGUAGE.md`:
   ```md
   ## <topic>
   | Term | Definition | Aliases to avoid |
   |------|------------|------------------|
   | **CanonicalTerm** | one-line | synonym1, synonym2 |
   ```
5. Surface: **"Canonical terms. Anything missing or wrong?"**
6. On confirm: write file + record terms in persistence.
7. Reference glossary in phase 3+ (PRDs, prompts, review).

**Stop:** every load-bearing term has one canonical definition. File written.

**Anti-pattern:** glossary as afterthought — wrong vocab cements into code.

---

## Phase 3 — PRD (modules, interfaces, tests)

**Symptom prevented:** code ships not matching plan. **Cause:** no module-level plan; AI invents structure that drifts.

1. Query for related architecture notes.
2. Read relevant code paths — confirm current state, don't assume.
3. Draft PRD:
   - **Goal** — one sentence
   - **Non-goals** — what stays out
   - **Modules touched** — name, current interface, proposed change, rationale (glossary terms)
   - **New modules** — name, interface signature, behaviors, deep-module hierarchy slot
   - **Test plan** — per behavior: which interface boundary, what assertion
   - **Open questions** — anything unresolvable from prior context/code
4. Only canonical glossary terms. No synonyms, no inventions.
5. Surface: **"Modules + interfaces match your model?"**
6. Resolve open questions one at a time. Cycle to phase 1/2 if PRD reveals missing decision/term.
7. On confirm: write `PRD-<topic>.md`, record summary.
8. Small changes: skip file, write GitHub issues with same module/interface/test specificity.

**Stop:** every module change names module/interface/behavior added/test that proves it. User confirmed.

**Anti-pattern:** PRD names files but not interfaces. Interfaces = contract; files = scaffolding.

---

## Phase 5 — Design interfaces, delegate implementation

**Symptom prevented:** brain fatigue, code shipping faster than holdable. **Cause:** reviewing implementations doesn't scale; designing interfaces does.

1. For new modules from phase 3-4, design interface deliberately:
   - Accepts what?
   - Returns what?
   - Invariants guaranteed?
   - Explicitly *not* doing what?
2. Treat impl as **gray box**: trust contract, test from outside. Review inside only for high-stakes (auth, payment, security, financial).
3. For shallow-module thickets blocking phase 4: identify clusters, wrap behind one deep-module interface, move tests to new boundary, delete obsolete inner interfaces.
4. Beck: **invest in design every day**. Every commit invests or divests.
5. Update persistence after non-trivial architectural changes: record new module purpose+interface, link to PRD/design concept, forget stale notes.

**Stop:** new modules expose simple interfaces, hide impl, testable from outside, compose without leaking dependencies.

**Anti-pattern:** line-by-line review of deep module you trust. Time sink.

---

## Loop — when to revisit

| Symptom | Phase |
|---|---|
| AI built wrong thing | 1 align |
| Terms drift, AI verbose | 2 glossary |
| Ships code not matching plan | 3 PRD |
| Big diffs, late tests, type errors | 4 TDD |
| Brain fatigue, can't hold system | 5 design+delegate |
| AI contradicts prior decisions | 0 orient |

After every phase: record decision so next session starts with more context.

## Greenfield checklist

| Phase | Query | Record |
|---|---|---|
| 0 orient | 2-3 phrasings, full text top-3 | — |
| 1 align | prior design concepts, architecture | design concept |
| 2 glossary | prior glossary | each term |
| 3 PRD | architecture for affected modules | PRD summary |
| 4 TDD | (commits cite decision IDs) | — |
| 5 design+delegate | stale architecture | new module interface; forget stale |
