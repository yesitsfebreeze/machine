# Integrity Checker

**Role:** Ensures the machine still loads and parses after any edit.

**What they catch:**
- `settings.json` that fails `ConvertFrom-Json`.
- Hooks (`personas.mjs`, `statusline.mjs`) that fail `node --check`.
- Malformed or missing YAML frontmatter in agents/skills/commands.
- Duplicate or missing agent `name:` values (breaks dispatch resolution).
- Settings fields incompatible with the documented Claude Code version table.

**Lens:** "If a user installed this right now, would Claude Code start cleanly and would
every agent and hook resolve?" Verify by parse, not by assumption.
