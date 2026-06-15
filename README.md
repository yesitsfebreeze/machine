---
instructions: If you are a Claude Code agent, install the machine as a plugin -> `/plugin marketplace add yesitsfebreeze/machine` then `/plugin install machine@machine`, then run `/oil` to write the project layer.
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
![Skills](https://img.shields.io/badge/skills-25-3b82f6?style=for-the-badge)
![Built for](https://img.shields.io/badge/built%20for-Claude%20Code-d97757?style=for-the-badge&logo=anthropic)

</div>

---

## What is this?

The machine is a Claude Code **plugin** you install into any repository. It gives
your agent a full operating system: a roster of specialist subagents, an on-demand
skill toolbelt, a quality gate, a review panel, and a compounding memory — none of
it tied to a particular codebase. Installing namespaces every component under
`machine:` (`machine:gate`, `machine:coder`, …).

When you run `/oil`, it reads the repo and writes a thin **project layer** that
teaches the portable machine your stack, your laws, and your vocabulary. The same
plugin behaves like a Rust expert in one repo and a frontend expert in the next.

```
the machine plugin   the portable payload  ← installed + updated via /plugin
/.machine/              the project layer     ← never shipped; /oil writes it per repo
```

The split is the whole idea: **one portable machine**, **one per-repo brain**.

---

## ✨ What it can do

- **🤖 Bare-bones agent core.** Four agents stay loaded: the eager-generalist
  `default`, the `drill` driver, and `manager-tdd` / `manager-ddd` for
  greenfield and legacy implementation. The generalist drives everything else.
- **🧰 Small skill core.** Seven skills load by default: `coder` and `clean` for
  building and polishing; `gate` for the quality gate; `personas` for review;
  `codex-review` for second-AI checks; `drill` — the single entry point that drives
  work and runs session/bootstrap bring-up; and `oil` for the project layer.
- **📦 The `mine/` addon kit.** The rest of the toolbelt — the `expert-*` and
  `builder-*` agents, `manager-spec` / `-strategy` / `-git` / `-docs` / etc.,
  `tool-ast-grep`, `workflow-*`, `foundation-*`, `ref-*`, `specialists`, `improve`,
  `parallel`, `perf-gate`, `learn`, `caveman`, `trello`, and more — lives in
  `mine/` at the repo root. Nothing there is loaded until you slot it in; `/oil`
  can scan it and suggest the pieces that fit a given repo.
- **🧠 Compounding memory ([`kern`](https://github.com/yesitsfebreeze/kern)).** A
  per-directory memory daemon, reached over MCP, that remembers *why* past
  decisions were made and surfaces them before you re-decide. Knowledge is
  ingested on the fly and recalled at session start.
- **🕸️ Fleet coordination (`mesh`).** kern's coordination sibling: a per-directory
  MCP daemon that gives a fleet of parallel agents a live roster, atomic
  cross-process claims/locks (with leases, queues, and self-healing on agent
  death), and durable point-to-point/broadcast/topic mail with per-agent read
  cursors. Same install idiom and on-disk shape as `kern`.
- **🎭 Persona review panel.** A data-driven panel (defined in `/.machine/personas/`)
  spawns one reviewer per lens in parallel, then synthesizes a ship verdict —
  tuned to each repo's real risk surface.
- **🎼 Orchestrator mode.** Spawn background subagents for agreed units of work,
  persist one durable state file per agent, validate each result through the gate
  and the persona panel, and surface a "needs your approval" footer — all without
  blocking the conversation.
- **✅ Language-agnostic quality gate.** `/gate` detects the stack and runs
  format, lint, tests, and build in one pass — pass/fail before any commit.
- **⚙️ Hooks and modes.** Session-start context injection (drill bring-up), a live status
  line, and a post-task persona-review nudge — wired through the active Node hooks.
- **🔁 Clean lifecycle.** Install and update are the plugin system's job
  (`/plugin install` / `/plugin update`); `/oil` re-indexes the project layer
  whenever the repo changes shape.

---

## 🚀 Quick start

1. **Install the plugin.** Add the marketplace and install:
   ```
   /plugin marketplace add yesitsfebreeze/machine
   /plugin install machine@machine
   ```
2. **Bring up.** Run `/drill`. On a fresh repo its bring-up makes one idempotent
   pass: it bootstraps every dependency (`kern`, `mesh`, the `git-fs` plugin, the MCP
   prerequisites), wires the status line and API keys, then oils the project layer
   (`/oil`) — writing `/.machine/` from your actual code (identity, stack, glossary,
   persona panel) — and then drives. (`just bootstrap` runs the same deps install from
   a terminal.)
3. **Work.** The default generalist drives the work — slotting specialists from
   `mine/` when a domain needs one — recalls prior decisions from `kern`, gates
   quality before commits, and offers the review panel after non-trivial changes.

> Pull machine updates with `/plugin update machine`. Re-run `/drill` bring-up (or
> `just bootstrap`) to reinstall deps or re-wire config, or `/oil` alone to re-index `/.machine/`
> after the project changes shape.

> **Bundled MCP servers.** The plugin ships five servers in its `.mcp.json`:
> [`kern`](https://github.com/yesitsfebreeze/kern) (memory),
> `mesh` (fleet coordination — roster, atomic claims, durable mail; a
> zero-dependency Node script — no build),
> [`context7`](https://context7.com) (current library docs — set `CONTEXT7_API_KEY`),
> `pdf-reader` (PDF extraction via `npx @sylphx/pdf-reader-mcp`),
> and [`context-mode`](https://github.com/mksglu/context-mode) (keep large output out
> of context via the `ctx_*` tools — runs via `npx`, needs Node >=22.5.0).
>
> **Companion plugin** (live, installed separately — the machine routes to it but
> doesn't vendor it, since it ships no standalone binary):
> [`git-fs`](https://github.com/yesitsfebreeze/git-fs) (per-session virtual git
> filesystem). Install with `/plugin marketplace add yesitsfebreeze/git-fs` then
> `/plugin install git-fs@git-fs`.
>
> **One-shot setup.** From a checkout of this repo, `just bootstrap` (or
> `bash scripts/bootstrap.sh`) installs and verifies every dependency above in one
> idempotent pass — `kern`, `mesh`, the `git-fs` plugin, and the MCP prerequisites.

---

## 🧠 How it works

- **The default agent is an eager generalist.** Whole-toolbelt, bias-to-verify; it
  prefers ground truth over guessing and drives most work itself, slotting a
  specialist from `mine/` when a domain decision needs one.
- **Machine law is always on.** Root-cause fixes over patches, one clean
  implementation per change, single source of truth, glossary discipline, and
  durable memory in `kern`.
- **Project law lives in `/.machine/`.** `agent.md` (identity + hard rules),
  `project.md` (stack, key paths), `glossary.csv`, and `personas/` — all written
  by `/oil`, never shipped with the portable payload.
- **Knowledge loads on demand.** Skills pull their deep context only when a
  decision calls for it, keeping every turn token-cheap. The `mine/` kit goes one
  step further — its agents and skills are not loaded at all until slotted in.

---

## 🗂️ Layout

```
.claude-plugin/          plugin + marketplace manifests (plugin.json, marketplace.json)
.claude/                 the portable machine (the plugin payload)
├── agents/              4 core agents — resolved by `name:` frontmatter
├── skills/              8 core skills — one dir each, `name:` matches the dir
├── hooks/               Node ESM hooks + hooks.json plugin manifest
├── rules/               coding standards (project-scope; not shipped by the plugin)
├── output-styles/       comm modes (machine, einstein)
└── settings.json        self-host hook wiring, env, default agent

mine/                       the addon kit (curated; not loaded — slot in via /oil)
├── agents/             extracted specialist agents
├── skills/             extracted skills
└── hooks/              extracted hook scripts

/.machine/                  the project layer (written per repo by /oil)
├── agent.md             this repo's identity + hard rules
├── project.md           stack, key paths, vision
├── glossary.csv         the repo's vocabulary
├── personas/            the review panel
└── sessions/            drill roster / ledger (ephemeral)
```

---

<div align="center">



**Built for [Claude Code](https://claude.com/claude-code).**
Portable. Project-agnostic. Specializes itself.

</div>





11 12 13  21 22 23 ▶31 32 33 
14 15 16  24 25 26  34◀35 36 
17 28 19  27 28 39  37 38 39 

41 42 43  51 52 53  61 62 63 
44 45 46  54 55 56  64 65 66 
47 48 49  57 58 59  67 68 69 

71 72 73  81 82 83  91 92 93 
74 75 76  84 85 86  94 95 96 
77 78 79  57 58 89  97 98 99 

