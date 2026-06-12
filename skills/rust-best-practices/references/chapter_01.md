---
tags: [skill, material, streaming, code, pipeline, agent]
---

# Chapter 1 — Coding Styles & Idioms

## 1.1 Borrow > Clone

Prefer `&T` over `T.clone()`. Performance recommendation.

### When to clone
- Need owned copy AND preserve original (immutable snapshot)
- `Arc`/`Rc` pointers
- Cross-thread share (usually `Arc`)
- API expects owned data
- Avoid massive refactor of non-perf-critical code
- Cached return: `fn get_config(&self) -> Config { self.cached_config.clone() }`

### Clone traps
- Auto-clone in loops `.map(|x| x.clone())` — use `.cloned()` / `.copied()` at end
- Cloning large `Vec<T>` / `HashMap<K,V>`
- Clone due to bad API — fix lifetimes
- Prefer `&[T]` over `Vec<T>` / `&Vec<T>`
- Prefer `&str` over `String` / `&String`
- Clone of reference arg → caller should pass ownership instead

```rust
// ✅ borrow
fn process(name: &str) { println!("Hello {name}"); }
let user = String::from("foo");
process(&user);

// ❌ redundant clone
fn process_string(name: String) { ... }
process(user.clone());
```

## 1.2 Pass by value (Copy)

Small + cheap-to-copy types pass by value. `Copy` trait makes explicit.

### Pass by value when
- Type implements `Copy` (`u32`, `bool`, `f32`, small structs)
- Move cost negligible

### Derive `Copy` on own types when
- All fields `Copy`
- Small (≤24 bytes / 2-3 words)
- Plain data, no heap (no `Vec`/`String`)

❗ Rust arrays stack-allocated. Copyable if inner is `Copy`, but stack overflow risk. See chapter 3.

Primitive sizes: `i8/u8`=1B, `i16/u16`=2B, `i32/u32/f32/char`=4B, `i64/u64/f64`=8B, `i128/u128`=16B, `bool`=1B.

```rust
// ✅
#[derive(Debug, Copy, Clone)]
struct Point { x: f32, y: f32, z: f32 }

// ❌
#[derive(Debug, Clone)]
struct BadIdea { age: i32, name: String }  // String not Copy
```

Enums: `Copy` if tags + atoms + all payloads `Copy`. ❗ Enum size = largest variant.

## 1.3 `Option<T>` & `Result<T,E>` patterns

Rust 1.65: `let Some(x) = … else { … }` for early returns when missing case is expected.

### Use `match` when
- Pattern match against inner `T` and `E`
- Type transformed to something complex (e.g. `Result<T,E>` → `Result<Option<T>, E>`)

### Use `let PATTERN = EXPR else { DIVERGE; }` when
- Diverging code doesn't need failed pattern info
- Want to break/continue inside loop

### Use `if let PATTERN = EXPR else { DIVERGE; }` when
- Diverging code needs extra computation

❗ Don't care about `Err` value → use `?` to propagate.

### Bad
- `match { Ok(t) => Some(t), Err(_) => None }` — use `.ok()` / `.ok_or()` / `.ok_or_else()`
- `if let` when divergent is default/precomputed → use `let … else`
- `unwrap()` / `expect()` outside tests

## 1.4 Prevent early allocation

Functions like `or`, `map_or`, `unwrap_or`, `ok_or` evaluate args eagerly. If allocation needed, use `_else` variant.

```rust
// ✅
x.ok_or_else(|| ParseError::ValueAbsent(format!("val {x}")))
x.map_or_else(|e| format!("Error: {e}"), |v| v.len())
x.parse_to_option_vec.unwrap_or_else(Vec::new)

// ❌
x.ok_or(ParseError::ValueAbsent(format!("val {x}")))  // formats every call
x.map_or(format!("Error"), |v| v.len())
```

### Map err
```rust
x.inspect_err(|err| tracing::error!("fn: {err}"))
 .map_err(|err| GeneralError::from(("fn", err)))?;
```

## 1.5 `iter()` vs `for`

```rust
// for
let mut sum = 0;
for x in 0..=10 { if x % 2 == 0 { sum += x + 1; } }

// iter
let sum: i32 = (0..=10).filter(|x| x % 2 == 0).map(|x| x + 1).sum();
```

### Prefer `for` when
- Early exits (`break`, `continue`, `return`)
- Simple iteration with side-effects (logging, IO — though `inspect`/`inspect_err` exist)
- Readability beats chaining

### Prefer iterators when
- Transforming collections / `Option`/`Result`
- Compose multiple steps elegantly
- No early exit
- Need `.enumerate`, `.windows`, `.chunks`
- Combine multi-source data without intermediate collections

❗ Iterators are LAZY. `.iter`, `.map`, `.filter` do nothing until consumer (`.collect`, `.sum`, `.for_each`). Lazy = fused into one loop at compile time.

### Anti-patterns
- Don't chain unformatted (rustfmt handles)
- Don't chain if unreadable
- Avoid needless `.collect`/allocate just to throw away
- Prefer `iter` over `into_iter` unless ownership needed
- Prefer `iter` over `into_iter` when inner is `Copy` (`Vec<i32>`)
- Prefer `.sum()` over `.fold()` for summing — specialized + optimized

## 1.6 Comments: context, not clutter

> Context is for *why*, not *what* or *how*.

Well-named code speaks for itself. Few/no comments common in good codebases.

### Good
- Safety: `// SAFETY: ptr is valid and non-null per @fn xyz`
- Perf: `// Fast inverse square root approximation`
- ADR/Design link: `// PERF: Generating per-subgraph caused TLS startup latency on macOS. See [ADR-123](link)`

### Bad
- Wall-of-text explanations → use `///` doc instead
- Obvious narration `// increment i by 1`

### Break long functions over commenting them

Long comment explaining "what/how/each step" → split function. Better readability + testability.

```rust
// ✅
fn process_request(request: T) -> Result<(), Error> {
    validate_request_headers(&request)?;
    let payload = decode_payload(&request);
    authorize(&payload)?;
    dispatch_to_handler(payload)
}
```

### TODOs as issues, not comments

Filed in Github/Jira. Reference issue in code: `// See issue #123: support hyper 2.0`.

### Comments rot

Don't trust blindly. Read in context. Wrong/outdated → fix or remove. Misleading comment worse than none.

Prefer: link ADR/Design Doc, move to `/// doc` (testable via `cargo doc`), break into named functions.

## 1.7 Use declarations (imports)

Standard order:
1. `std` (`core`, `alloc`)
2. External crates (Cargo.toml `[dependencies]`)
3. Workspace crates
4. `super::`
5. `crate::`

```rust
use std::sync::Arc;

use chrono::Utc;
use juniper::{FieldError, FieldResult};

use broker::database::PooledConnection;

use super::schema::{Context, Payload};
use crate::models::Event;
```

`rustfmt.toml`:
```toml
reorder_imports = true
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
```

❗ Rust 1.88: needs nightly rustfmt for reorder — `cargo +nightly fmt`.
