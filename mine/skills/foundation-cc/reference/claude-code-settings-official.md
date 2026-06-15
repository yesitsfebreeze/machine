# Claude Code Settings - Official Documentation Reference

Source: https://code.claude.com/docs/en/settings

## Settings Hierarchy

Precedence (highest to lowest):

1. Enterprise managed settings (`managed-settings.json` — Windows: `C:\ProgramData\ClaudeCode\`, macOS: `/Library/Application Support/ClaudeCode/`, Linux: `/etc/claude-code/`)
2. Command-line arguments
3. Local project settings: `.claude/settings.local.json` (personal, gitignored)
4. Shared project settings: `.claude/settings.json` (team, version-controlled)
5. User settings: `~/.claude/settings.json` (personal, all projects)

## Valid Top-Level Fields

```json
{
 "model": "claude-sonnet-4-5-20250929",
 "permissions": {},
 "hooks": {},
 "disableAllHooks": false,
 "env": {},
 "statusLine": {},
 "outputStyle": "",
 "cleanupPeriodDays": 30,
 "sandbox": {},
 "enabledPlugins": {},
 "enableAllProjectMcpServers": false,
 "enabledMcpjsonServers": [],
 "disabledMcpjsonServers": [],
 "includeCoAuthoredBy": true,
 "apiKeyHelper": "",
 "forceLoginMethod": "",
 "effortLevel": "high",
 "disableBypassPermissionsMode": false
}
```

Version compatibility for newer fields lives in `.claude/rules/coding-standards.md`.

## Permissions

Modes: `default`, `plan`, `acceptEdits`, `dontAsk`, `bypassPermissions`.

```json
{
 "permissions": {
 "defaultMode": "default",
 "allow": ["Read", "Glob", "Grep", "Bash(git status:*)", "Bash(git log:*)"],
 "ask": ["Bash(rm:*)", "Bash(sudo:*)"],
 "deny": ["Read(~/.ssh/**)", "Bash(rm -rf /:*)"],
 "additionalDirectories": []
 }
}
```

Rule shape: `Tool` or `Tool(pattern)`. `deny` beats `ask` beats `allow`.

## Environment Variables

```json
{
 "env": {
 "NODE_ENV": "development",
 "PYTHONPATH": "./src",
 "DEBUG": "true"
 }
}
```

## MCP Servers

MCP servers are configured in `.mcp.json` at the project root (NOT in settings.json):

```json
{
 "mcpServers": {
 "kern": { "command": "kern", "args": ["mcp"] },
 "context7": {
 "command": "npx",
 "args": ["@upstash/context7-mcp"],
 "env": { "CONTEXT7_API_KEY": "$CONTEXT7_KEY" }
 }
 }
}
```

settings.json controls which apply: `enableAllProjectMcpServers`, `enabledMcpjsonServers`, `disabledMcpjsonServers`. MCP tool permissions use the standard permissions lists with `mcp__server__tool` names.

## Hooks

Hook events: SessionStart, UserPromptSubmit, PreToolUse, PermissionRequest, PostToolUse, PostToolUseFailure, Notification, SubagentStart, SubagentStop, Stop, PreCompact, SessionEnd.

Hook handler types: "command" (shell command), "prompt" (LLM evaluation), "agent" (subagent with tool access).

Timeout unit: seconds. Defaults: 600 for command, 30 for prompt, 60 for agent.

```json
{
 "hooks": {
 "PreToolUse": [
 {
 "matcher": "Bash",
 "hooks": [
 { "type": "command", "command": ".claude/hooks/block-rm.sh", "timeout": 10 }
 ]
 }
 ],
 "PostToolUse": [
 {
 "matcher": "Write|Edit",
 "hooks": [
 { "type": "command", "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/lint-check.sh", "timeout": 30 }
 ]
 }
 ],
 "Stop": [
 {
 "hooks": [
 { "type": "prompt", "prompt": "Check if all tasks are complete: $ARGUMENTS", "timeout": 30 }
 ]
 }
 ]
 }
}
```

Use `$CLAUDE_PROJECT_DIR` in hook commands, never absolute paths.

## Sub-agents and Plugins

There is NO `subagents` or `plugins` settings block. Sub-agents are markdown files in `.claude/agents/`; plugins are enabled per-plugin via `enabledPlugins` and managed with the `/plugin` command.

## Management

- `/config` — interactive settings UI in the REPL
- `claude config get|set|add|remove|list` — CLI (user scope with `--global`, else project)
- Edit the JSON files directly; validate JSON after every edit

## Best Practices

- Version-control `.claude/settings.json`; keep `.claude/settings.local.json` gitignored
- Never commit secrets — use `env` indirection or apiKeyHelper
- Principle of least privilege in `allow` lists
