---
name: tool-ast-grep
description: >
  AST-based structural code search, security scanning, and refactoring using
  ast-grep (sg CLI) with pattern matching and code transformation across 40+
  languages. Use for structural search or codemod operations.
license: Apache-2.0
compatibility: Designed for Claude Code
allowed-tools: Read, Grep, Glob, Bash(sg:*), Bash(ast-grep:*), mcp__plugin_machine_context7__resolve-library-id, mcp__plugin_machine_context7__query-docs
user-invocable: false
metadata:
  version: "1.2.0"
  category: "tool"
  modularized: "true"
  status: "active"
  updated: "2026-01-11"
  tags: "ast, refactoring, code-search, lint, structural-search, security, codemod"
  related-skills: "workflow-testing, foundation-quality, rust-best-practices"
  context: "fork"
  agent: "Explore"

# extension: Triggers
triggers:
  keywords: ["ast", "refactoring", "code search", "lint", "structural search", "security", "codemod", "ast-grep"]
---

# AST-Grep Integration

ast-grep (sg) matches patterns against the AST, not text — no false positives from comments, strings, or name fragments. Use it over regex whenever the question is structural.

## When to Use

- Code patterns regex cannot capture (nested calls, scoped matches)
- Multi-file refactoring with semantic awareness; API migrations
- Security scanning for vulnerability patterns (SQLi, XSS, secrets)
- Enforcing style rules at the syntax level

## Core Commands

```bash
sg -p '<pattern>' [path] --lang <lang>     # structural search
sg -p '<old>' -r '<new>' [path]            # transform (preview; -U applies)
sg scan [--config sgconfig.yml] [path]     # rule-based scan (auto-discovers sgconfig.yml at root)
sg test rules/                             # validate rule files
```

Install: `brew install ast-grep` / `npm install -g @ast-grep/cli` / `cargo install ast-grep`.

## Pattern Essentials

- `$NAME` — capture one AST node (e.g. `const $NAME = require($PATH)`)
- `$$$ARGS` — variadic capture (e.g. `function $NAME($$$ARGS)`)
- `$$_` — anonymous match when the value is not needed

YAML rules add relational power: `inside`, `has`, `follows`, `precedes`, `not`, and composite `all`/`any` — full grammar in modules/pattern-syntax.md.

Supported languages: Python, JavaScript, TypeScript, Go, Rust, Java, Kotlin, C, C++, Ruby, Swift, C#, PHP, Scala, Elixir, Lua, HTML, Vue, Svelte, and 30+ more.

## Module Map

| Need | File |
|---|---|
| Full pattern/rule grammar, fix syntax | modules/pattern-syntax.md |
| Refactoring and migration codemods | modules/refactoring-patterns.md |
| OWASP security rule templates | modules/security-rules.md |
| Per-language pattern catalogs | modules/language-specific.md |
| Workflows, CI/CD recipes, troubleshooting | references/examples.md |
| Official docs links, ecosystem | references/reference.md |

Current upstream documentation: resolve `ast-grep` via Context7 (`mcp__plugin_machine_context7__resolve-library-id` → `mcp__plugin_machine_context7__query-docs`).

## Machine Integration

- `Bash(sg:*)` and `Bash(ast-grep:*)` are in this skill's allowed-tools; the Explore agent prefers structural search over text search where patterns are structural.
- expert-refactoring (large-scale codemods), expert-security (vulnerability scans), and manager-quality (complexity checks) lean on this skill.

<!-- machine:evolvable-start id="rationalizations" -->
## Common Rationalizations

| Rationalization | Reality |
|---|---|
| "Regex search is good enough for finding code patterns" | Regex matches text, not structure. Searching for `function` matches comments, strings, and variable names. AST matching is structural. |
| "I will review the codemod output manually, dry-run is unnecessary" | Codemods at scale produce hundreds of changes. Manual review without dry-run misses edge cases in obscure files. |
| "This rule is too strict, I will disable it globally" | Global disable hides all violations, including legitimate ones. Disable per-file with inline comments and justification. |
| "ast-grep is overkill for a small refactor" | Small refactors across many files are exactly where ast-grep prevents missed occurrences. Manual find-replace misses variants. |
| "The pattern works in the playground, it will work on the codebase" | Playground uses a single file context. Codebase patterns encounter language variants, file encodings, and edge cases. Always dry-run. |

**Rule of 500**: For changes affecting more than 500 lines, automated structural search is more reliable than manual inspection. ast-grep provides the structural guarantee that regex cannot.

<!-- machine:evolvable-end -->

<!-- machine:evolvable-start id="red-flags" -->
## Red Flags

- Codemod applied without dry-run output reviewed first
- ast-grep rule disabled globally instead of per-file with justification
- Regex used for structural code search when ast-grep pattern would be more precise
- Pattern tested only in playground, not on actual codebase files
- Codemod output not verified by running tests after application

<!-- machine:evolvable-end -->

<!-- machine:evolvable-start id="verification" -->
## Verification

- [ ] Dry-run output reviewed before applying codemod (show dry-run results)
- [ ] Tests pass after codemod application (show test output)
- [ ] Pattern validated on at least 3 representative files before bulk application
- [ ] No ast-grep rules disabled globally without documented justification
- [ ] Codemod diff reviewed for unintended changes (show diff summary)

<!-- machine:evolvable-end -->
