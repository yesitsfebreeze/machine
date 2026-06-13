---
instructions: If you are a claude code agent, read this file instead -> **`.claude/INSTRUCTIONS.md`**
---

# The machine

A **portable agent setup** — project-agnostic workflow, integrations, agents, skills, and
hooks. It carries nothing about any specific codebase. Installed, it lives at
`<project>/.claude/`. Everything that diverges per project lives in `/.proj/` (a sibling of
`.claude/`), which `/oil-me` regenerates by reading the current repo.

```
this repo's root   the machine        ← the install payload; becomes <project>/.claude/
/.proj/            the project layer  ← never copied; /oil-me writes it per repo
```