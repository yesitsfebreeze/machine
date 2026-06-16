---
name: mcp-plugin
description: >
  Author an external, install-anywhere Claude Code plugin that bundles an MCP
  server. Produces the full layout (.claude-plugin/plugin.json, .mcp.json,
  marketplace.json), wires the portable path variables ${CLAUDE_PLUGIN_ROOT},
  ${CLAUDE_PROJECT_DIR}, and ${CLAUDE_PLUGIN_DATA} so the plugin runs on any
  machine with no absolute host paths, validates with `claude plugin validate`,
  and ships a one-line `/plugin install` flow. Use when asked to build, package,
  or distribute an MCP server as a Claude Code plugin. Trigger: "/mcp-plugin",
  "build an mcp plugin", "package my mcp server", "make a claude plugin",
  "distribute mcp", "marketplace plugin".
metadata:
  version: "1.0.0"
  category: "builder"
  status: "active"
  updated: "2026-06-16"
  tags: "mcp, plugin, marketplace, distribution, claude-code, packaging"
---

# mcp-plugin — package an MCP server as an install-anywhere plugin

Goal: a self-contained plugin directory anyone installs with one `/plugin`
command, whose MCP server starts automatically and runs unchanged on any host.
"Works everywhere" is won or lost on **path portability** — the variables below
are the whole game. Never hardcode an absolute host path.

## The portable path variables (the core of this skill)

Resolved by the harness in `.mcp.json`, hook commands, LSP, and monitor
configs. Same substitution set: `${CLAUDE_PLUGIN_ROOT}`, `${CLAUDE_PLUGIN_DATA}`,
`${CLAUDE_PROJECT_DIR}`, `${user_config.*}`, and any `${ENV_VAR}`.

| Variable | Resolves to | Use for | Never |
|----------|-------------|---------|-------|
| `${CLAUDE_PLUGIN_ROOT}` | absolute path to the plugin's install dir (this changes on every update) | bundled server binary, scripts, default config, `cwd` | write runtime state here — the old dir is GC'd ~7 days after an update |
| `${CLAUDE_PLUGIN_DATA}` | `~/.claude/plugins/data/{id}/` (persists across updates) | runtime state, caches, installed `node_modules`, db files | bundling read-only assets — those go under ROOT |
| `${CLAUDE_PROJECT_DIR}` | the project root (same value hooks get in `$CLAUDE_PROJECT_DIR`) | project-local scripts/config the plugin acts on | assuming it equals ROOT — they differ |

Quoting rule: in **shell-form** hook/monitor commands and any `command` string,
wrap in double quotes — `"${CLAUDE_PLUGIN_ROOT}/scripts/x.sh"`. In **exec-form**
hooks, pass it via `args` as one element so it needs no quoting. In `.mcp.json`
`command`/`args`/`env`/`cwd` values, bare `${CLAUDE_PLUGIN_ROOT}/...` is fine
(no shell splitting there).

Portability test before shipping: `grep -rn '/home/\|/Users/\|C:\\\\\|/opt/\|/usr/local' .` over the plugin must come back empty. Every path that points inside the bundle must go through a variable.

## Directory layout

```
my-plugin/
├── .claude-plugin/
│   └── plugin.json          # manifest — only place this dir holds
├── .mcp.json                # MCP server definitions (or inline in plugin.json)
├── skills/                  # optional <name>/SKILL.md
├── agents/  hooks/  commands/   # optional, all at ROOT (not under .claude-plugin/)
├── bin/                     # optional executables added to Bash PATH while enabled
├── README.md  LICENSE  CHANGELOG.md
```

Hard rule: only `plugin.json` lives in `.claude-plugin/`. Every component dir
(`skills/`, `agents/`, `hooks/`, `.mcp.json`, …) sits at the plugin root.

## Step process

1. **Pick the transport.** stdio (a local process — most portable), or remote
   HTTP/SSE. For a bundled node/python server prefer stdio launched via a runner
   that needs no global install (`npx -y <pkg>`, `uvx <pkg>`, or a binary under
   `bin/`). This is what makes it install-anywhere.
2. **Scaffold.** `claude plugin init <name>` then choose the `mcp` template (or
   `channel` for an MCP channel server). It writes a valid skeleton you edit.
3. **Write `plugin.json`** (template below). `name` is the only required field;
   set `version` so users get updates only when you bump it (omit → every commit
   is a new version). Add `$schema` for editor validation.
4. **Write `.mcp.json`** (template below). Reference the bundled server through
   `${CLAUDE_PLUGIN_ROOT}`; put any writable state path under
   `${CLAUDE_PLUGIN_DATA}`.
5. **Surface user settings** via `${user_config.*}` for tokens/endpoints the user
   provides, or read `${ENV_VAR}` with a default (`${API_KEY:-}`) so a missing
   secret does not crash startup.
6. **Validate.** `claude plugin validate ./my-plugin --strict` (CI-grade; treats
   unknown/misspelled fields as errors). Fix until clean.
7. **Local dev loop** (no marketplace): drop the plugin folder (with its
   `.claude-plugin/plugin.json`) under a `skills/` dir of an existing project and
   it loads next session as `<name>@skills-dir` — no install step. Edits to a
   `SKILL.md` apply live; changes to `.mcp.json`/`hooks/`/`agents/` need
   `/reload-plugins` or a restart.
8. **Publish.** Push to GitHub, add a `marketplace.json` (template below) at repo
   root in `.claude-plugin/`. Users then run the one-liner under Install.

## Templates

`.claude-plugin/plugin.json`:

```json
{
  "$schema": "https://json.schemastore.org/claude-code-plugin-manifest.json",
  "name": "my-plugin",
  "displayName": "My Plugin",
  "version": "1.0.0",
  "description": "What it does in one line.",
  "author": { "name": "you", "url": "https://github.com/you" },
  "repository": "https://github.com/you/my-plugin",
  "license": "MIT",
  "keywords": ["mcp", "..."],
  "defaultEnabled": false
}
```

`.mcp.json` — bundled stdio server, portable everywhere:

```json
{
  "mcpServers": {
    "my-server": {
      "command": "npx",
      "args": ["-y", "@you/my-mcp-server"],
      "cwd": "${CLAUDE_PLUGIN_ROOT}",
      "env": {
        "STATE_DIR": "${CLAUDE_PLUGIN_DATA}",
        "API_KEY": "${MY_API_KEY:-}"
      }
    }
  }
}
```

Bundled binary/script variant: `"command": "${CLAUDE_PLUGIN_ROOT}/servers/my-server"`.
Remote variant: `{ "type": "http", "url": "https://...", "headers": { ... } }`.

`.claude-plugin/marketplace.json` (at the repo root that hosts the plugin):

```json
{
  "name": "my-marketplace",
  "owner": { "name": "you", "url": "https://github.com/you" },
  "plugins": [
    {
      "name": "my-plugin",
      "source": { "source": "github", "repo": "you/my-plugin" },
      "description": "What it does."
    }
  ]
}
```

A `source` may also be a local path (`"./my-plugin"`) when the plugin lives in
the same repo as the marketplace.

## Install flow (what you hand the user)

```
/plugin marketplace add you/my-marketplace-repo
/plugin install my-plugin@my-marketplace
```

Headless / CI: `claude plugin install my-plugin@my-marketplace [--scope project|local]`.
The bundled MCP server's per-server approval prompt is the same one a project
`.mcp.json` triggers.

## Gotchas

- MCP servers a plugin declares go through the same per-server approval as a
  project `.mcp.json` — document that the user must approve on first run.
- `CLAUDE.md` at the plugin root is NOT loaded as context. Ship instructions as
  a skill, not a CLAUDE.md.
- For runtime deps (e.g. `npm install`'d `node_modules`), install into
  `${CLAUDE_PLUGIN_DATA}` on first run and run the server against that — ROOT is
  wiped on update.
- Marketplace plugins are copied into `~/.claude/plugins/cache`; symlinks
  pointing outside the marketplace are dropped for security. Bundle, don't link
  out.
- `--strict` validation in CI catches a field that is one or two chars off a real
  one before publish.

## After shipping

Offer `/personas` review of the plugin, and record the plugin's name, transport,
and install one-liner in kern (`mcp__kern__ingest`) so future sessions recall it.
