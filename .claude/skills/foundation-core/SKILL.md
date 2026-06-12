---
name: foundation-core
description: >
  Foundational principles for working in the machine: delegation to specialist
  agents, a language-agnostic quality gate, progressive disclosure, and modular
  skill structure. Use when creating agents/skills or deciding how to route work.
license: Apache-2.0
compatibility: Designed for Claude Code
allowed-tools: Read, Grep, Glob, mcp__context7__resolve-library-id, mcp__context7__get-library-docs
user-invocable: false
metadata:
  version: "3.0.0"
  category: "foundation"
  status: "active"
  updated: "2026-06-12"
  tags: "foundation, core, delegation, quality-gate, progressive-disclosure, modular"
progressive_disclosure:
  enabled: true
  level1_tokens: 100
  level2_tokens: 5000
triggers:
  keywords: ["delegation", "agent", "skill", "quality gate", "progressive disclosure", "modular", "routing"]
  agents:
    - "manager-spec"
    - "manager-ddd"
    - "manager-strategy"
    - "manager-quality"
    - "builder-agent"
    - "builder-skill"
---

# Foundation Core

The principles that keep work in the machine correct, delegated, and verifiable.
The canonical authority is `.claude/agents/default.md` (Machine law) plus the
project layer in `/.proj/`; this skill explains the *why* behind four recurring
decisions. It never overrides Machine law or project law.

## 1. Delegate to the right specialist

The default agent is an orchestrator, not a do-everything implementer. Route
domain work to the specialist that owns it rather than executing it inline —
delegation buys an independent context and domain focus.

- Agents resolve by their `name:` frontmatter, not by path. The dispatch table
  lives in `default.md`; the catalog of who-does-what is there and in
  `/.proj/project.md`.
- Spawn specialists with the Agent tool. When several pieces of work are
  independent, dispatch them in one message so they run in parallel (see
  `superpowers:dispatching-parallel-agents`).
- Direct execution is appropriate for typo/format fixes, single-file config
  edits, result synthesis, git operations, and scratch work — anything where a
  specialist adds no independence or expertise.

## 2. Pass a quality gate before "done"

Every non-trivial change answers five questions before it is called complete.
The dimensions are language-agnostic; the concrete commands come from the
project's own toolchain via `/gate` (it detects fmt/lint/test/build, or reads
the exact commands from `/.proj/project.md`).

- **Tested** — the behavior is covered and the suite passes.
- **Readable** — names and structure make intent obvious; the linter is clean.
- **Unified** — formatting and imports match the project; one clean
  implementation, no leftover duplicate.
- **Secured** — no introduced vulnerability, unbounded input, or concurrency
  hazard; escalate to `expert-security` when in doubt.
- **Trackable** — a clear commit message; the change is traceable to its reason.

Verify by evidence, not by feeling: run the check and quote the output
(`superpowers:verification-before-completion`). In this repo "build" means
configuration integrity — hooks pass `node --check`, `settings.json` parses, and
every agent/skill reference resolves.

## 3. Progressive disclosure

Deliver knowledge in tiers so context budget is spent on what the task needs.

- A `SKILL.md` is the entry point: a quick reference plus an implementation
  guide, kept compact. Deep material moves to `modules/<topic>.md`, reached only
  when the task demands it.
- `examples.md` holds working samples; `references/` holds external links and
  API detail. Cross-reference instead of inlining.

## 4. Modular skill structure

Keep each skill self-describing and small.

- Directory `name` must match the `name:` frontmatter, or dispatch loads the
  wrong (or no) skill.
- One responsibility per skill; split overflow into `modules/` rather than
  growing one file unboundedly.
- Single source of truth: each fact lives in exactly one file; reference with
  `@file` instead of copying.

## Works well with

- `default.md` — Machine law and the live dispatch table (authority).
- `/.proj/project.md`, `/.proj/agent.md` — project facts, paths, and project law.
- `foundation-cc` — Claude Code authoring detail for skills, agents, hooks,
  plugins, and settings.
- `foundation-quality` — the quality-gate mechanics in depth.
- `/gate` — the stack-detecting pre-commit check.

## Red flags

- The default agent implementing specialist-domain code instead of delegating.
- Claiming "done" without running the gate and quoting its output.
- A skill directory whose `name:` frontmatter no longer matches its path.
- The same fact copied into two files instead of referenced once.
