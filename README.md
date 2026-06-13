---
instructions: If you are a claude code agent, read this file instead -> **`.claude/INSTRUCTIONS.md`**
---

<div align="center">

# 🛠️ the machine

**A portable, project-agnostic [Claude Code](https://claude.com/claude-code) setup —
agents, skills, hooks, and memory that travel between repos and specialize
themselves to each one.**

[![Stars](https://img.shields.io/github/stars/yesitsfebreeze/machine?style=for-the-badge&logo=github&color=f5c518)](https://github.com/yesitsfebreeze/machine/stargazers)
[![Forks](https://img.shields.io/github/forks/yesitsfebreeze/machine?style=for-the-badge&logo=github&color=6e7681)](https://github.com/yesitsfebreeze/machine/network/members)
[![Last commit](https://img.shields.io/github/last-commit/yesitsfebreeze/machine?style=for-the-badge&logo=git&color=8b5cf6)](https://github.com/yesitsfebreeze/machine/commits/main)
[![Issues](https://img.shields.io/github/issues/yesitsfebreeze/machine?style=for-the-badge&logo=github&color=ef4444)](https://github.com/yesitsfebreeze/machine/issues)

![Agents](https://img.shields.io/badge/agents-23-22c55e?style=for-the-badge)
![Skills](https://img.shields.io/badge/skills-24-3b82f6?style=for-the-badge)
![Built for](https://img.shields.io/badge/built%20for-Claude%20Code-d97757?style=for-the-badge&logo=anthropic)

</div>

---

## What is this?

The machine is a drop-in `.claude/` payload you copy into any repository. It gives
your Claude Code agent a full operating system: a roster of specialist subagents,
an on-demand skill toolbelt, a quality gate, a review panel, and a compounding
memory — none of it tied to a particular codebase.

When you run `/oil-me`, it reads the repo it lives in and writes a thin **project
layer** that teaches the portable machine your stack, your laws, and your
vocabulary. The same payload behaves like a Rust expert in one repo and a
frontend expert in the next.

```
this repo's root   the machine        ← the install payload; becomes <project>/.claude/
/.proj/            the project layer  ← never copied; /oil-me writes it per repo
```

The split is the whole idea: **one portable machine**, **one per-repo brain**.

---

## ✨ What it can do

- **🤖 23 specialist agents, auto-dispatched.** Domain work routes itself to the
  right expert — `expert-backend`, `expert-frontend`, `expert-security`,
  `expert-performance`, `expert-debug`, `expert-devops`, `expert-testing`,
  `expert-refactoring`; the `manager-*` line for TDD, DDD, specs, strategy, git,
  docs, quality, and project setup; `builder-*` for authoring new agents, skills,
  and plugins; plus `researcher`, `plan-auditor`, and an adversarial
  `evaluator-active`.
- **🧰 24 on-demand skills.** A toolbelt that loads only when needed: `coder`,
  `clean`, `improve`, `simplify` for building and polishing; `gate`,
  `code-review`, `perf-gate` for quality; `tool-ast-grep` for structural
  search/codemod; reference kits (`foundation-cc`, `rust-best-practices`,
  `ref-owasp-checklist`, `ref-testing-pyramid`, `ref-git-workflow`); and
  orchestration (`orchestrate`, `parallel`, `personas`).
- **🧠 Compounding memory ([`kern`](https://github.com/yesitsfebreeze/kern)).** A
  per-directory memory daemon, reached over MCP, that remembers *why* past
  decisions were made and surfaces them before you re-decide. Knowledge is
  ingested on the fly and recalled at session start.
- **🎭 Persona review panel.** A data-driven panel (defined in `/.proj/personas/`)
  spawns one reviewer per lens in parallel, then synthesizes a ship verdict —
  tuned to each repo's real risk surface.
- **🎼 Orchestrator mode.** Spawn background subagents for agreed units of work,
  persist one durable state file per agent, validate each result through the gate
  and the persona panel, and surface a "needs your approval" footer — all without
  blocking the conversation.
- **✅ Language-agnostic quality gate.** `/gate` detects the stack and runs
  format, lint, tests, and build in one pass — pass/fail before any commit.
- **⚙️ Hooks and modes.** Session-start context injection, a live status line, an
  ultra-compressed "caveman" output mode, and orchestrator resume — wired through
  6 Node hooks.
- **🔁 One lifecycle command.** `/oil-me` installs the machine, pulls updates, and
  re-indexes the project layer whenever the repo changes shape.

---

## 🚀 Quick start

1. **Drop it in.** Copy this repo's payload into your project so it lives at
   `<your-project>/.claude/`.
2. **Oil the machine.** Run `/oil-me`. It installs (or updates) the machine and
   writes `/.proj/` by reading your actual code — identity, stack, glossary, and a
   persona panel.
3. **Work.** The default agent routes to specialists, recalls prior decisions from
   `kern`, gates quality before commits, and offers the review panel after
   non-trivial changes.

> Already installed? Re-run `/oil-me` any time to pull machine updates or
> re-index `/.proj/` after the project changes.

---

## 🧠 How it works

- **The default agent is an eager generalist.** Whole-toolbelt, bias-to-verify; it
  prefers ground truth over guessing and routes domain decisions to specialists.
- **Machine law is always on.** Root-cause fixes over patches, one clean
  implementation per change, single source of truth, glossary discipline, and
  durable memory in `kern`.
- **Project law lives in `/.proj/`.** `agent.md` (identity + hard rules),
  `project.md` (stack, key paths), `glossary.csv`, and `personas/` — all written
  by `/oil-me`, never shipped with the portable payload.
- **Knowledge loads on demand.** Specialists and skills pull their deep context
  only when a decision calls for it, keeping every turn token-cheap.

---

## 🗂️ Layout

```
.claude/                 the portable machine (this is the install payload)
├── agents/              23 dispatch agents — resolved by `name:` frontmatter
├── skills/              24 skills — one dir each, `name:` matches the dir
├── hooks/               6 Node ESM hooks (session start, statusline, personas…)
├── rules/               coding standards for instruction files
├── output-styles/       comm modes (machine, caveman)
├── settings.json        hook wiring, env, default agent
└── INSTRUCTIONS.md      the lifecycle protocol /oil-me executes

/.proj/                  the project layer (written per repo by /oil-me)
├── agent.md             this repo's identity + hard rules
├── project.md           stack, key paths, vision
├── glossary.csv         the repo's vocabulary
├── personas/            the review panel
└── sessions/            orchestrator state board (ephemeral)
```

---

<div align="center">

**Built for [Claude Code](https://claude.com/claude-code).**
Portable. Project-agnostic. Specializes itself.

</div>
