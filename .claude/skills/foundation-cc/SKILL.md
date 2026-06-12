---
name: foundation-cc
description: >
  Canonical Claude Code authoring kit covering Skills, sub-agents, plugins, slash commands,
  hooks, memory, settings, sandboxing, headless mode, and advanced agent patterns.
  Use when creating Claude Code extensions or configuring Claude Code features.
license: Apache-2.0
compatibility: Designed for Claude Code
allowed-tools: Read, Write, Edit, Grep, Glob, mcp__context7__resolve-library-id, mcp__context7__get-library-docs
user-invocable: false
metadata:
  version: "5.0.0"
  category: "foundation"
  status: "active"
  updated: "2026-01-11"
  modularized: "false"
  tags: "foundation, claude-code, skills, sub-agents, plugins, slash-commands, hooks, memory, settings, sandboxing, headless, agent-patterns"
  aliases: "foundation-cc"

# extension: Progressive Disclosure
progressive_disclosure:
  enabled: true
  level1_tokens: 100
  level2_tokens: 5000

# extension: Triggers
triggers:
  keywords: ["skill", "agent", "plugin", "slash command", "hook", "sandbox", "headless", "memory", "settings", "claude code", "sub-agent", "agent pattern", "orchestration", "delegation"]
  agents:
    - "builder-agent"
    - "builder-skill"
    - "builder-plugin"
  phases:
    - "plan"
    - "run"
    - "sync"
---

# Claude Code Authoring Kit

Reference set for authoring Claude Code Skills, sub-agents, plugins, slash commands, hooks, memory, settings, sandboxing, and headless usage. The detail lives in reference/ — this file is the index plus the facts needed for routing.

## Documentation Index

Core Features:

- reference/claude-code-skills-official.md - Agent Skills creation and management
- reference/claude-code-sub-agents-official.md - Sub-agent development and delegation
- reference/claude-code-plugins-official.md - Plugin architecture and distribution
- reference/claude-code-discover-plugins-official.md - Finding and installing plugins
- reference/claude-code-plugin-marketplaces-official.md - Marketplace setup and distribution
- reference/claude-code-custom-slash-commands-official.md - Command creation and orchestration

Configuration:

- reference/claude-code-settings-official.md - Configuration hierarchy and management
- reference/claude-code-memory-official.md - Context and knowledge persistence
- reference/claude-code-hooks-official.md - Event-driven automation
- reference/claude-code-iam-official.md - Permissions and access control

Advanced:

- reference/claude-code-sandboxing-official.md - Security isolation
- reference/claude-code-headless-official.md - Programmatic and CI/CD usage
- reference/claude-code-devcontainers-official.md - Containerized environments
- reference/claude-code-cli-reference-official.md - Command-line interface
- reference/claude-code-statusline-official.md - Custom status display
- reference/advanced-agent-patterns.md - Orchestration and context engineering

## Quick Reference

- Skills: model-invoked, `.claude/skills/<name>/SKILL.md` (project) or `~/.claude/skills/` (personal). Three-level progressive disclosure: metadata (~100 tokens) → body (<5K tokens) → on-demand resources. Keep under 500 lines; third-person description with trigger terms.
- Sub-agents: markdown in `.claude/agents/`, invoked via `Agent(subagent_type=...)`. Own 200K context; cannot spawn sub-agents; do user interaction before delegation. Frontmatter: name, description (use PROACTIVELY for auto-delegation), tools CSV, model.
- Plugins: bundles with `.claude-plugin/plugin.json`; can ship commands, agents, skills, hooks, MCP servers. Manage via `/plugin`.
- Commands: user-invoked `/name`; `$ARGUMENTS`, `$1`, `$2`, `@file` refs. Thin routing wrappers per coding-standards.
- Hooks: events in settings.json (see hooks doc for the full event list and handler types).
- Memory: CLAUDE.md hierarchy + `.claude/rules/*.md` + @imports (see memory doc).
- Settings: precedence enterprise-managed > CLI > local > project > user (see settings doc).
- Headless: `claude -p` with `--allowedTools`; always restrict tools in CI.

## The Machine's Authoring Conventions

- Agents resolve by `name:` frontmatter (must be unique); `skills:` refs must resolve to real skill dirs — verify after every rename.
- Validate settings.json JSON after every edit; every registered hook needs an existing script file.
- Use `$CLAUDE_PROJECT_DIR` in hook commands, never absolute paths.
- Core builder agents: builder-skill (skills), builder-agent (sub-agents), builder-plugin (plugins). manager-spec / manager-ddd / manager-quality for spec-driven work.

<!-- machine:evolvable-start id="rationalizations" -->
## Common Rationalizations

| Rationalization | Reality |
|---|---|
| "I will use Bash sed instead of Edit, it is faster" | Edit is the preferred tool for accuracy and review. Bash sed errors are silent and hard to trace. |
| "This hook does not need a timeout, it finishes quickly" | Hooks without timeouts can hang the entire session. Always set an explicit timeout. |
| "I can put all logic in CLAUDE.md, rules are overkill" | CLAUDE.md has a 40K character limit. Rules load conditionally and scale without bloating the prompt. |
| "Settings.json changes are low risk" | Incorrect settings.json breaks hooks, permissions, and model routing. Validate the JSON after every edit. |
| "I will skip progressive disclosure, all content is needed" | Loading 5K tokens for every skill wastes 67% of context. Level 1 metadata is sufficient for routing. |
| "This skill does not need allowed-tools, Claude will figure it out" | Missing allowed-tools means the skill silently inherits all tools. Explicit is safer than implicit. |

<!-- machine:evolvable-end -->

<!-- machine:evolvable-start id="red-flags" -->
## Red Flags

- CLAUDE.md exceeds 40,000 characters
- Hook registered in settings.json without a corresponding script file
- Skill frontmatter uses space-separated allowed-tools instead of comma-separated
- Agent definition uses YAML array for tools instead of CSV string
- settings.json contains hardcoded absolute paths instead of $CLAUDE_PROJECT_DIR
- Progressive disclosure disabled for a skill that exceeds 3000 tokens

<!-- machine:evolvable-end -->

<!-- machine:evolvable-start id="verification" -->
## Verification

- [ ] CLAUDE.md character count is under 40,000 (show wc -c output)
- [ ] settings.json is valid JSON (show json validation output)
- [ ] Every hook in settings.json has a matching script file in .claude/hooks/
- [ ] All skill frontmatter uses CSV format for allowed-tools
- [ ] Agent frontmatter uses CSV for tools and YAML array for skills
- [ ] All metadata values in skill frontmatter are quoted strings
- [ ] $CLAUDE_PROJECT_DIR used instead of absolute paths in hook commands

<!-- machine:evolvable-end -->
