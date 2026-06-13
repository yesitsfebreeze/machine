# The machine

A **portable agent setup** — project-agnostic workflow, integrations, agents, skills, and
hooks. It carries nothing about any specific codebase. Installed, it lives at
`<project>/.claude/`. Everything that diverges per project lives in `/.proj/` (a sibling of
`.claude/`), which `/bootstrap` regenerates by reading the current repo.

```
this repo's root   the machine        ← the install payload; becomes <project>/.claude/
/.proj/            the project layer  ← never copied; bootstrap writes it per repo
```

---

**Claude coding agent, pointed here to install or update the machine?**

The full protocol lives in **`CLAUDE_INSTRUCTIONS.md`**. Derive its raw URL from the URL you
fetched this README from (swap `README.md` → `CLAUDE_INSTRUCTIONS.md`), fetch it, and follow
it. It will have you place a self-updating copy at `/.proj/CLAUDE_INSTRUCTIONS.md`.
