---
name: rust-best-practices
description: Idiomatic Rust per Apollo GraphQL handbook. Use when (1) writing new Rust, (2) reviewing/refactoring, (3) borrow vs clone, (4) error handling, (5) perf, (6) tests/docs.
when_to_use: Writing/reviewing/refactoring Rust. Skip non-Rust, high-level design.
license: MIT
compatibility: Rust 1.70+, Cargo
metadata:
  author: apollographql
  version: "1.1.0"
allowed-tools: Bash(cargo:*) Bash(rustc:*) Bash(rustfmt:*) Bash(clippy:*) Read Write Edit Glob Grep
tags: [skill, material, code, pipeline, perf, testing]
---

# Rust Best Practices

Apollo handbook: https://github.com/apollographql/rust-best-practices

## Chapter index (load on demand, parallel reads OK)

- `references/chapter_01.md` — Styles & idioms: borrow vs clone, Copy, Option/Result, iterators, comments
- `references/chapter_02.md` — Clippy & linting: config, lints, workspace setup
- `references/chapter_03.md` — Perf mindset: profiling, redundant clones, stack vs heap, zero-cost
- `references/chapter_04.md` — Errors: Result vs panic, thiserror vs anyhow, hierarchies
- `references/chapter_05.md` — Tests: naming, one assertion, snapshots
- `references/chapter_06.md` — Generics & dispatch: static vs dyn, trait objects
- `references/chapter_07.md` — Type-state pattern
- `references/chapter_08.md` — Comments vs docs
- `references/chapter_09.md` — Pointers, Send/Sync, thread safety

## Quick reference

**Borrow & ownership**: prefer `&T` over `.clone()`. `&str` over `String`, `&[T]` over `Vec<T>` in params. Small Copy types (≤24B) by value. `Cow<'_, T>` when ambiguous.

**Errors**: `Result<T,E>` for fallible. No `panic!` in prod. No `unwrap()`/`expect()` outside tests. `thiserror` for libs, `anyhow` for binaries. `?` over match chains.

**Perf**: bench with `--release`. `cargo clippy -- -D clippy::perf`. No clone in loops. `.iter()` over `.into_iter()` for Copy. Iterators over manual loops, avoid intermediate `.collect()`.

**Lint**: `cargo clippy --all-targets --all-features --locked -- -D warnings`. Watch `redundant_clone`, `large_enum_variant`, `needless_collect`. Use `#[expect(clippy::lint)]` over `#[allow]` with justification.

**Tests**: descriptive names (`process_should_return_error_when_input_empty`). One assertion when possible. Doc tests for public API. `cargo insta` for snapshots.

**Generics**: prefer generics (static dispatch) for hot paths. `dyn Trait` only for heterogeneous. Box at API boundaries, not internally.

**Type-state**: encode states in types to catch invalid ops at compile time:
```rust
struct Connection<State> { _state: PhantomData<State> }
struct Disconnected; struct Connected;
impl Connection<Connected> { fn send(&self, data: &[u8]) {} }
```

**Docs**: `//` = *why* (safety, workarounds, rationale). `///` = *what/how* for public APIs. Every TODO links issue: `// TODO(#42): ...`. Libs: `#![deny(missing_docs)]`.
