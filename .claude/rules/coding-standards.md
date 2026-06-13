---
description: Coding standards for the machine's instruction documents and configuration files
globs: .claude/**/*.md, .claude/**/*.yaml, .proj/**/*.md, CLAUDE.md
---

# Coding Standards

Coding standards specific to this machine's instruction and configuration files. General coding conventions are not included as Claude already knows them.

## Language Policy

All instruction documents must be in English:
- CLAUDE.md
- Agent definitions (.claude/agents/**/*.md)
- Skill definitions (.claude/skills/**/*.md)
- Hook scripts (.claude/hooks/**/*.py, *.sh)
- Project layer documents (/.proj/**/*.md, glossary.csv)

User-facing documentation may use multiple languages:
- README.md, CHANGELOG.md
- User guides, API documentation

## File Size Limits

CLAUDE.md must not exceed 40,000 characters.

When approaching limit:
- Move detailed content to .claude/rules/
- Use @import references
- Keep only core identity and hard rules in CLAUDE.md

## Content Restrictions

Prohibited in instruction documents:
- Code examples for conceptual explanations
- Flow control as code syntax
- Decision trees as code structures
- Emoji characters (except output styles)
- Time estimates or duration predictions

## Duplicate Prevention

Single source of truth principle:
- Each piece of information exists in exactly one location
- Use references (@file) instead of copying content
- Update source file, not copies

## No Slash Commands

Slash commands are retired in this machine; the `.claude/commands/` directory
must not exist. All workflow logic lives in skills under `.claude/skills/<name>/`,
invoked directly via the Skill tool.

## Claude Code Version Compatibility

Settings fields introduced by specific Claude Code versions:

| Field | Version | Notes |
|-------|---------|-------|
| `effortLevel` | v2.1.110 | Sets CLAUDE_CODE_EFFORT_LEVEL; values: low/medium/high/xhigh/max |
| `disableBypassPermissionsMode` | v2.1.111 | Prevents agents from using bypassPermissions mode when true |
| `Bash(timeout=N)` | v2.1.110 | Per-command Bash timeout in ms; max 600,000ms |
| `requiredMinimumVersion` / `requiredMaximumVersion` | v2.1.163 | Managed setting: enforce a Claude Code version range at startup |
| `fallbackModel` | v2.1.166 | Up to three fallback models tried in order when the primary is unavailable |
| `enforceAvailableModels` | v2.1.175 | Managed setting: constrain Default model resolution to an allowlist |

When adding new settings fields, update `.claude/settings.json`
and this compatibility table.

## Claude Code Capability Notes

Non-settings harness facts the machine relies on (verified against the official
changelog):

| Capability | Version | Notes |
|------------|---------|-------|
| `post-session` hook | v2.1.169 | Lifecycle hook runs after session ends, before workspace deletion |
| `--safe-mode` / `CLAUDE_CODE_SAFE_MODE` | v2.1.169 | Disables customizations (hooks, skills, settings overrides) |
| `ultracode` keyword | v2.1.161 | Workflow trigger keyword (renamed from `workflow`) |
| Stop/SubagentStop `additionalContext` | v2.1.163 | Hooks may return `hookSpecificOutput.additionalContext` |
| Nested subagents | v2.1.172 | Sub-agents can spawn up to 5 levels deep |

## Paths Frontmatter

Use paths frontmatter for conditional rule loading:

```yaml
---
paths: "**/*.py,**/pyproject.toml"
---
```

This ensures rules load only when working with matching files.
