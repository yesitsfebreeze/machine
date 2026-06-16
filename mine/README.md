# mine — curated addon kit

Things we like and use, kept out of the active machine so the core stays
bare-bones. Nothing here is registered with or loaded by Claude Code.

## Slotting an item back in
1. Copy the agent file into `.claude/agents/`, or the skill folder into `.claude/skills/`.
2. Add its path to the matching array (`agents` / `skills`) in `.claude-plugin/plugin.json`.
3. For a hook, restore its entry in `.claude/hooks/hooks.json`.

## /mine — slot the right tools
`/mine` surveys this kit (the mine graph) and the current repo, matches the
best-fit agents/skills/hooks, and slots them in (copy out + register in
`.claude-plugin/plugin.json` / `hooks.json`), recording each decision in kern so
sessions compound. `/oil` owns the project layer; `/mine` equips the machine.

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
