---
name: gate
description: >
  Fast pre-commit quality gate. Runs format check, lint, tests, and build in one
  pass and reports a pass/fail table. Toolchain-agnostic: detects the project's
  stack (or reads /.machine/project.md for the exact commands). Use before any commit,
  or whenever you want a quick "is the tree green?" check. Trigger: "/gate",
  "quality gate", "pre-commit check", "is it green", "run the checks".
metadata:
  version: "2.0.0"
  category: "workflow"
  status: "active"
  updated: "2026-06-12"
  tags: "gate, quality, lint, format, test, build, pre-commit, ci"
---

# Gate — pre-commit quality gate

Lightweight, fast validation before a commit. One pass, report a table. This is the
cheap gate; it does **not** replace deep review (`/personas`, `/code-review`) or any
domain sign-off (e.g. real-time/safety, security).

## Where the commands come from

Use the project's own commands, in this priority order:

1. **`/.machine/project.md`** — if it lists a "Quality gate" / build / test line, use
   exactly those. This is the source of truth per repo.
2. **CI** — mirror `.github/workflows/*` (or other CI config) so a green local gate
   means green CI.
3. **Detect from the manifest** — first match wins:

   | Manifest | Format | Lint | Test | Build |
   |----------|--------|------|------|-------|
   | `Cargo.toml` | `cargo fmt --all -- --check` | `cargo clippy --all-targets -- -D warnings` | `cargo test` | `cargo build` |
   | `package.json` | `prettier --check .` | `eslint .` | `npm test` | `tsc --noEmit` / `npm run build` |
   | `pyproject.toml` | `ruff format --check` | `ruff check` | `pytest` | — |
   | `go.mod` | `gofmt -l .` | `go vet ./...` | `go test ./...` | `go build ./...` |
   | `CMakeLists.txt` | — | — | `ctest` | `cmake --build build` |

   No recognized manifest → report "unknown toolchain" and ask the user for the
   commands (then suggest adding them to `/.machine/project.md`).

Honor any **extra project-specific check** named in `/.machine/project.md` (e.g. an
embedded/`no_std` build, a wasm target, a schema validation) — those catch what the
generic checks miss; never skip them.

## Run

Run the checks from the repo root. Launch the read-only ones concurrently (separate
Bash calls in one message) and collect results.

- **`--fix`** — apply safe auto-fixes first (formatter, lint `--fix`), then re-run.
  Never auto-"fix" failing tests or a broken build — those need diagnosis
  (the `expert-debug` agent).

## Report

```
## Gate: PASS
| Check   | Status | Time  |
|---------|--------|-------|
| Format  | PASS   | 0.4s  |
| Lint    | PASS   | 6.1s  |
| Tests   | PASS   | 14.2s |
| Build   | PASS   | 3.0s  |
```

On failure, list the concrete offenders under the table (file:line for lint, test
names for failures) — never just "lint failed".

## Boundaries & after a failure

- Gate proves the tree compiles, formats, lints, and tests pass. It does **not**
  prove domain correctness, real-time safety, or security — follow with `/personas`
  / `/code-review` for those.
- `--fix` clears formatting + mechanical lints. Test/build failures: stop and find
  the root cause — never silence a test or `#[allow]`/eslint-disable a lint to go
  green. Green gate → clear to commit.
