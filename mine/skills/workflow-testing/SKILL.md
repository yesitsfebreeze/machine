---
name: workflow-testing
description: >
  Testing workflow for the machine: characterization tests for legacy code,
  specification tests for greenfield, and the test-quality checklist. Use when
  writing tests or measuring coverage.
license: Apache-2.0
compatibility: Designed for Claude Code
allowed-tools: Read, Write, Edit, Bash(pytest:*), Bash(ruff:*), Bash(npm:*), Bash(npx:*), Bash(node:*), Bash(jest:*), Bash(vitest:*), Bash(go:*), Bash(cargo:*), Bash(mix:*), Bash(uv:*), Bash(bundle:*), Bash(php:*), Bash(phpunit:*), Grep, Glob, mcp__plugin_machine_context7__resolve-library-id, mcp__plugin_machine_context7__query-docs
user-invocable: false
metadata:
  version: "3.0.0"
  category: "workflow"
  status: "active"
  updated: "2026-06-12"
  tags: "workflow, testing, ddd, tdd, coverage"
progressive_disclosure:
  enabled: true
  level1_tokens: 100
triggers:
  keywords: ["testing", "characterization tests", "coverage", "DDD", "TDD", "behavior preservation", "test"]
  agents: ["manager-ddd", "manager-tdd", "expert-testing", "expert-performance", "expert-refactoring"]
---

# Workflow Testing

Specialists own the method: `manager-tdd` (red-green-refactor),
`expert-debug` (bugs). Coverage strategy: `ref-testing-pyramid`.
Quality gate: `/gate`. This skill is the testing-specific checklist.

## Legacy code — characterization (PRESERVE)

- Write tests that capture what the code does now, not what it should do.
- Snapshot input/output pairs for complex paths as regression guards.
- Confirm the characterization suite is green before changing anything.
- Refactor in small steps; keep the suite green throughout.

## Greenfield — specification

- Derive each test from a domain rule or invariant; name it for the behavior.
- Organize by domain concept (aggregate, entity, value object).
- Specify behavior in domain language; keep implementation detail out of tests.
- Write the test before the implementation.

## Rationalizations

| Rationalization | Reality |
|---|---|
| "Already covered by integration tests" | Integration tests catch different bugs than unit tests. |
| "Mocking the DB is too hard, skip it" | Hard-to-test means a missing abstraction boundary. |
| "80% coverage is good enough" | Coverage targets are floors; the missing paths are error handling. |
| "Utility functions don't need tests" | Utilities are the most reused code; a bug propagates everywhere. |
| "Passed locally, CI will pass" | Trust CI output, not local runs. |
| "Flaky tests are normal, re-run" | Fix the flakiness or quarantine the test explicitly. |

## Red flags

- Coverage dropped after a feature addition.
- `skip`/`t.Skip()` without a linked issue.
- Auto-generated test names (test_1) instead of behavior-descriptive.
- No test on the error/failure branch of a new function.
- Test imports the concrete implementation instead of the interface.

## Verification

- [ ] Suite passes, zero failures — paste command output.
- [ ] Coverage meets threshold for changed code.
- [ ] Error paths have dedicated cases, not just the happy path.
- [ ] No new flaky tests (re-run to confirm stability).
- [ ] Each test owns its fixtures/temp dirs.
- [ ] Race detector passed for concurrent code.
