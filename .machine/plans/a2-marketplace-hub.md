# Plan a2 — machine as the single marketplace hub

## Goal
Make `machine`'s `.claude-plugin/marketplace.json` the single Claude Code plugin
marketplace hub for yesitsfebreeze. One `/plugin marketplace add yesitsfebreeze/machine`
exposes every useful plugin. Reference-only (github sources) — no vendoring, zero
duplication. `kern` stays solo (its own marketplace, not folded in). The legacy
`stack` hub is retired by deleting the repo.

## Decisions (agreed in drill)
- Hub model: **reference by github source** (no code copied into machine).
- Plugins listed in the hub: `machine` (self), `git-fs`, `split`. `kern` excluded
  (stays solo). `vicky`, `wiki` excluded.
- `stack`: **delete the repo** (irreversible; supersede by machine).

## Confirmed facts
- git-fs: real plugin, `yesitsfebreeze/git-fs`, `.claude-plugin/plugin.json` name
  `git-fs` v3.1.2 — "Copy-on-write overlay filesystem over the working directory,
  backed by a dedicated bare git store."
- split: real plugin, `yesitsfebreeze/split`, plugin.json name `split` v0.1.0 —
  "Fn-level code index MCP server."
- kern: real plugin, excluded by user.
- stack: marketplace-only repo (v0.3.0), the prior hub. To be deleted.
- gh token scopes lack `delete_repo`; deletion needs `gh auth refresh -s delete_repo`
  or web UI.

## Work units
1. **Edit `machine/.claude-plugin/marketplace.json`** — add two plugin entries
   (`git-fs`, `split`) as `{ source: github, repo: yesitsfebreeze/<name> }` alongside
   the existing `machine` entry. Keep `machine` first (install hub first). Preserve
   schema/ordering style. Update the top-level marketplace `description` to name it the
   hub.
2. **Update `machine/README.md`** — a short "Marketplace hub" section: how to add the
   hub and install each plugin from one place; note kern is separate; note stack is
   retired/superseded.
3. **Retire `stack`** — delete `github.com/yesitsfebreeze/stack` (driver-gated github
   action, run after the repo edits merge; requires delete_repo scope).

## Done-criteria
- `marketplace.json` validates and lists machine + git-fs + split with correct github
  sources.
- `/plugin marketplace add yesitsfebreeze/machine` then installing git-fs / split works
  conceptually (entries resolvable to real repos).
- README documents the single-hub install path.
- gate green.
- stack repo deleted (or scope-blocked and surfaced to user).

## Out of scope
- Vendoring any plugin code into machine.
- Touching kern, vicky, wiki.
- Changing machine's bundled mcpServers (mesh/context7/pdf-reader/context-mode stay
  inside the machine plugin).
