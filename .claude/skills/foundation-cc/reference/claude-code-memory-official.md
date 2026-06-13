# Claude Code Memory System - Official Documentation Reference

Source: https://code.claude.com/docs/en/memory

## Memory Files

Claude Code's file memory is CLAUDE.md, loaded automatically at session start:

1. Enterprise policy CLAUDE.md — managed location (Windows: `C:\ProgramData\ClaudeCode\`, macOS: `/Library/Application Support/ClaudeCode/`, Linux: `/etc/claude-code/`)
2. Project memory — `./CLAUDE.md` (version-controlled, team-shared); parent directories of cwd are also read (useful in monorepos)
3. User memory — `~/.claude/CLAUDE.md` (personal, all projects)

CLAUDE.md files in subdirectories load on demand when Claude reads files there. `CLAUDE.local.md` is deprecated — use imports instead.

## Rules Directory

`.claude/rules/*.md` holds modular instructions. Optional `paths` frontmatter scopes a rule to matching files only:

```yaml
---
paths: "**/*.py,**/pyproject.toml"
---
```

Keep CLAUDE.md under 40K characters (project law); push detail into rules and imports.

## Import Syntax

```markdown
@README.md          # relative
@docs/standards.md  # relative path
@~/.claude/my-project-instructions.md  # home directory
```

- Max import depth: 5 hops
- Imports are NOT evaluated inside code spans/blocks
- There is no conditional import syntax — imports are unconditional

## Managing Memory

- `/init` — bootstrap a CLAUDE.md for the current project
- `/memory` — open memory files in the editor
- `#` prefix — quick-add a memory (prompts for which file)

## The Machine's Memory Discipline

File memory (CLAUDE.md/rules) holds *instructions*. Durable project *knowledge* lives in kern, the per-cwd memory daemon:

- Recall: SessionStart digest + `mcp__kern__query`
- Capture: `mcp__kern__ingest` (required when kern responds; skip silently when down)
- Never use file-memory for durable knowledge; never duplicate kern content into CLAUDE.md

Machine command surface for memory-adjacent work: `/oil-me` (install/update/re-index /.machine), `/gate`, `/code-review`, `/improve`, `/personas`.

## Best Practices

- Be specific: "Use 2-space indentation" beats "format code properly"
- Structure as bullet points under descriptive markdown headings
- Review and prune memory periodically — stale instructions mislead
- Version-control project CLAUDE.md and .claude/rules/
