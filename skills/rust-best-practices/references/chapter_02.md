---
tags: [skill, lod, code, pipeline, perf, process]
---

# Chapter 2 — Clippy & Linting

Install: `rustup update && rustup component add clippy`. Verify: `cargo clippy -V`.
Docs: https://doc.rust-lang.org/clippy/usage.html

## 2.1 Why lint

Clippy catches: perf pitfalls, style, redundancy, potential bugs, non-idiomatic Rust.

## 2.2 Standard command

```bash
cargo clippy --all-targets --all-features --locked -- -D warnings
```

- `--all-targets`: lib, tests, benches, examples
- `--all-features`: enable all, auto-resolve conflicts
- `--locked`: requires Cargo.lock current; fix with `cargo update`
- `-D warnings`: warnings → errors

Optional: `-W clippy::pedantic` (strict, occasional false positives), `-W clippy::nursery` (in-development lints).

❗ Add to Makefile/Justfile/xtask/CI.

## 2.3 Important lints

| Lint | Why |
|------|-----|
| `redundant_clone` | Unnecessary clones, perf impact |
| `needless_borrow` | Redundant `&` |
| `map_unwrap_or` / `map_or` | Simplify nested Option/Result |
| `manual_ok_or` | Use `.ok_or_else` over `match` |
| `large_enum_variant` | Variant too big → suggests `Box` |
| `unnecessary_wraps` | Always-Ok/Some fn shouldn't return Result/Option |
| `clone_on_copy` | `.clone()` on Copy type (`u32`, `bool`) |
| `needless_collect` | Allocating iterator unnecessarily |

Full: https://rust-lang.github.io/rust-clippy/master/

## 2.4 Fix, don't silence

NEVER `#[allow(clippy::lint)]` unless:
- Truly understand why + reason it's better
- Document why ignored
- ❗ Use `#[expect(clippy::lint)]` not `#[allow]` — warns when lint no longer applies

```rust
// Faster matching preferred over size efficiency
#[expect(clippy::large_enum_variant)]
enum Message { Code(u8), Content([u8; 1024]) }
```

### False positives
1. Refactor to silence
2. **Local** override `#[expect(clippy::lint)]` + reason comment
3. Avoid global overrides (except core crate issues like Bevy)

## 2.5 Workspace / package lints in Cargo.toml

```toml
[lints.rust]
future-incompatible = "warn"
nonstandard_style = "deny"

[lints.clippy]
all = { level = "deny", priority = 10 }
redundant_clone = { level = "deny", priority = 9 }
manual_while_let_some = { level = "deny", priority = 4 }
pedantic = { level = "warn", priority = 3 }
```

Same shape under `[workspace.lints.rust]` / `[workspace.lints.clippy]` for workspace-wide.
