# AST-Grep Examples — Workflows and Integration

Pattern/rule examples live in the modules — this file holds only what they don't cover: end-to-end workflows, CI/CD recipes, performance, troubleshooting.

| Looking for | Go to |
|---|---|
| Meta-variables, relational/composite rules, fix syntax | modules/pattern-syntax.md |
| Rename, API migration, modernization codemods | modules/refactoring-patterns.md |
| OWASP security rules (SQLi, XSS, secrets, ...) | modules/security-rules.md |
| Per-language patterns (Python, TS, Go, Rust, Java) | modules/language-specific.md |
| Installation, docs links, module map | references/reference.md |

## Common Workflows

### Find and replace a function name

```bash
sg -p 'oldFunctionName($$$ARGS)' src/                                  # 1. find usages
sg -p 'oldFunctionName($$$ARGS)' -r 'newFunctionName($$$ARGS)' src/    # 2. preview (interactive)
sg -p 'oldFunctionName($$$ARGS)' -r 'newFunctionName($$$ARGS)' src/ -U # 3. apply (update all)
```

### Security audit

```bash
sg scan --config sgconfig.yml --severity error          # scan
sg scan --config sgconfig.yml --format sarif > out.sarif # SARIF for GitHub Security tab
```

### Refactoring session

```bash
sg -p '<pattern>' src/          # 1. identify sites
# 2. write rule yml with fix:
sg test rules/                  # 3. test the rule
sg scan -r my-rule.yml -U src/  # 4. apply
# 5. run the project's tests (/gate)
```

## CI/CD Integration

### GitHub Actions

```yaml
name: ast-grep scan
on: [push, pull_request]
jobs:
  scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: npm install -g @ast-grep/cli
      - run: sg scan --config sgconfig.yml --format github
```

### Pre-commit hook

```bash
#!/bin/sh
# .git/hooks/pre-commit
sg scan --config sgconfig.yml --severity error || {
  echo "ast-grep scan failed — fix errors before committing"; exit 1; }
```

GitLab/Jenkins: same single `sg scan --config sgconfig.yml` invocation inside the platform's job syntax.

## CLI Quick Reference

```bash
sg -p '<pattern>' [path] --lang <lang>   # search
sg -p '<old>' -r '<new>' [path]          # transform (add -U to write)
sg scan --config sgconfig.yml [path]     # rule scan (--format json|sarif|github)
sg test rules/                           # test rules
sg --help / sg <cmd> --help              # help
```

## Performance

```bash
sg -p '<pattern>' --lang python src/     # language filter narrows parsing
sg -p '<pattern>' --globs '!node_modules' --globs '!dist'  # exclude dirs
```

Parallel execution is the default; scope by path to cut wall time.

## Troubleshooting

- Pattern not matching → inspect the AST: `sg -p '<pattern>' --debug-query`; patterns match AST nodes, not text — whitespace is irrelevant, node kinds are not.
- Transformation wrong → preview without `-U` first; check meta-variable names match between pattern and rewrite.
- Rule file rejected → `sg test rules/` reports the failing rule; YAML `id`, `language`, and `rule` keys are mandatory.
