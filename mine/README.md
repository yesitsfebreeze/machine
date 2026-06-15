# mine — curated addon kit

Things we like and use, kept out of the active machine so the core stays
bare-bones. Nothing here is registered with or loaded by Claude Code.

## Slotting an item back in
1. Copy the agent file into `.claude/agents/`, or the skill folder into `.claude/skills/`.
2. Add its path to the matching array (`agents` / `skills`) in `.claude-plugin/plugin.json`.
3. For a hook, restore its entry in `.claude/hooks/hooks.json`.

## /oil integration (intended)
`/oil` can scan this folder when specializing a repo and suggest agents/skills
that fit the project, for the user to slot in.

## Layout
- `agents/` - extracted sub-agent definitions
- `skills/` - extracted skills (one folder each)
- `hooks/`  - extracted hook scripts

## Contents
### Agents
- builder-agent
- builder-plugin
- builder-skill
- evaluator-active
- expert-backend
- expert-debug
- expert-devops
- expert-frontend
- expert-performance
- expert-refactoring
- expert-security
- expert-testing
- manager-docs
- manager-git
- manager-project
- manager-quality
- manager-spec
- manager-strategy
- plan-auditor
- researcher

### Skills
- caveman
- codex-peer-review (vendored, MIT; opt-in via /assemble — needs OpenAI Codex CLI)
- foundation-cc
- foundation-core
- foundation-quality
- helper
- improve
- learn
- parallel
- perf-gate
- ref-git-workflow
- ref-owasp-checklist
- ref-testing-pyramid
- specialists
- tool-ast-grep
- trello
- workflow-testing
- workflow-thinking

### Hooks
- helper-suggest.mjs
- helper-trigger.mjs
