---
description: Coding standards for the machine's instruction documents and configuration files
globs: .claude/**/*.md, .claude/**/*.yaml, .proj/**/*.yaml, CLAUDE.md
---

# Coding Standards

Coding standards specific to this machine's instruction and configuration files. General coding conventions are not included as Claude already knows them.

## Language Policy

All instruction documents must be in English:
- CLAUDE.md
- Agent definitions (.claude/agents/**/*.md)
- Slash commands (.claude/commands/**/*.md)
- Skill definitions (.claude/skills/**/*.md)
- Hook scripts (.claude/hooks/**/*.py, *.sh)
- Configuration files (.proj/config/**/*.yaml)

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

## Thin Command Pattern

All slash command files MUST be thin routing wrappers (under 20 LOC body).

Rules:
- Commands route to a skill via the Skill tool -- they never contain workflow logic
- All workflow logic belongs in the skill body under `.claude/skills/<name>/`
- YAML frontmatter must include: description, argument-hint, allowed-tools (CSV string)
- Root commands may contain router tables but no implementation logic

Template:
```
---
description: [One-sentence action description]
argument-hint: "[Optional arg]"
allowed-tools: Skill
---

Use the Skill tool to invoke the matching skill with: [subcommand] $ARGUMENTS
```

Enforcement: keep command bodies under 20 LOC; all logic lives in the skill.

## Claude Code Version Compatibility

Settings fields introduced by specific Claude Code versions:

| Field | Version | Notes |
|-------|---------|-------|
| `effortLevel` | v2.1.110 | Sets CLAUDE_CODE_EFFORT_LEVEL; values: low/medium/high/xhigh/max |
| `disableBypassPermissionsMode` | v2.1.111 | Prevents agents from using bypassPermissions mode when true |
| `Bash(timeout=N)` | v2.1.110 | Per-command Bash timeout in ms; max 600,000ms |

When adding new settings fields, update `.claude/settings.json`
and this compatibility table.

## Paths Frontmatter

Use paths frontmatter for conditional rule loading:

```yaml
---
paths: "**/*.py,**/pyproject.toml"
---
```

This ensures rules load only when working with matching files.
