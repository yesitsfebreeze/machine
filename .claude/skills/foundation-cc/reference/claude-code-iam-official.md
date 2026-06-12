# Claude Code IAM & Permissions - Official Documentation Reference

Source: https://code.claude.com/docs/en/iam

## Permission Model

Claude Code is conservative by default: read-only tools (Read, Grep, Glob) run without approval; anything that mutates state or reaches the network (Bash, Write, Edit, WebFetch, WebSearch) prompts unless a rule or mode allows it.

There are NO roles, RBAC, tool-restriction objects, or compliance frameworks in Claude Code's configuration — access control is exactly: permission rules + permission modes + enterprise managed settings + hooks.

## Permission Rules

Rules live in the `permissions` block of settings.json (`allow` / `ask` / `deny` + `additionalDirectories`). `deny` beats `ask` beats `allow`. Rule shapes:

- `Tool` — the whole tool, e.g. `WebSearch`
- `Bash(git status:*)` — command prefix match (`:*` = any suffix)
- `Read(~/.ssh/**)`, `Edit(src/**)` — gitignore-style path patterns for file tools
- `WebFetch(domain:*.github.com)` — domain-scoped web access
- `mcp__server` or `mcp__server__tool` — MCP servers/tools (no wildcard inside MCP names)

```json
{
 "permissions": {
 "defaultMode": "default",
 "allow": ["Read", "Bash(git status:*)", "WebFetch(domain:docs.python.org)"],
 "ask": ["Bash(git push:*)"],
 "deny": ["Read(./.env*)", "Read(~/.ssh/**)", "Bash(sudo:*)"],
 "additionalDirectories": ["../shared-lib/"]
 }
}
```

## Permission Modes

`default` (prompt per first use), `plan` (read-only analysis), `acceptEdits` (auto-accept file edits), `dontAsk` (auto-accept everything allowed by rules), `bypassPermissions` (skip all prompts — blockable via `disableBypassPermissionsMode: true` in managed settings).

Set via `defaultMode`, the `/permissions` command, or `--permission-mode` CLI flag. Sub-agents may set `permissionMode` in their frontmatter.

## Enterprise Managed Settings

Organization-wide policy that users cannot override. File: `managed-settings.json` — Windows: `C:\ProgramData\ClaudeCode\`, macOS: `/Library/Application Support/ClaudeCode/`, Linux: `/etc/claude-code/`. Same schema as settings.json; sits at the top of the precedence chain. Typical use: force `deny` rules, disable bypassPermissions, pin `forceLoginMethod` and `apiKeyHelper`.

## Credential Management

- Login: Claude.ai (subscription) or Claude Console (API billing); `forceLoginMethod` pins one.
- `apiKeyHelper`: script that returns an API key/auth token at runtime (refresh interval via `CLAUDE_CODE_API_KEY_HELPER_TTL_MS`).
- Cloud providers: Bedrock (`CLAUDE_CODE_USE_BEDROCK=1`), Vertex (`CLAUDE_CODE_USE_VERTEX=1`) with provider-native IAM.
- Credentials are stored in the OS keychain where available; never commit keys to settings files.

## Auditing and Enforcement Hooks

For policy beyond static rules, use hooks: a `PreToolUse` hook can inspect any tool call and block it (exit code 2), and `PostToolUse` can log activity. This — not a fictional monitoring config — is the supported enforcement mechanism.

## Best Practices

- Least privilege: allow narrow command prefixes, not whole tools
- `deny` for secrets paths (`.env*`, key material) in shared project settings
- Version-control project permission rules so the team shares one policy
- Audit with `claude --debug` and hook-based logging
