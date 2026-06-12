---
tags: [skill, graphics, code, process, testing, ui]
---

# Chapter 8 — Comments vs Documentation

> Clear code beats clear comments. When *why* isn't obvious, comment plainly or link.

## 8.1 Difference

| Purpose | `// comment` | `/// doc` / `//! crate doc` |
|---|---|---|
| Describe *why* | ✅ tricky reasoning | ❌ |
| Describe API | ❌ | ✅ public interfaces, usage, errors, panics |
| Maintainable | 🚨 rots | ✅ tied to code, in `cargo doc`, runs tests |
| Visibility | local | exported |

## 8.2 When to use `//`

For things code can't express:
- Safety guarantees (start with `// SAFETY:`)
- Workarounds / optimizations
- Legacy / platform-specific (some replaceable with `#[cfg(..)]`)
- Links to ADRs / Design Docs
- Non-obvious gotchas / assumptions

```rust
// SAFETY: ptr is non-null + aligned by caller
unsafe { std::ptr::copy_nonoverlapping(src, dst, len); }

// CONTEXT: Reuse root cert store across subgraphs to avoid duplicate OS calls.
// See [ADR-12](link/to/adr-12)
```

## 8.3 Bad comments

- Restate obvious (`// increment by 1`)
- Stale-prone
- `TODO`s without action (no linked issue)
- Replaceable by better naming or smaller fns
- Outdated/ancient

## 8.4 Don't write "living documentation"

Comments rot, mislead, go stale, clutter. If something deserves to live beyond a PR:
- ADR (perf tradeoffs, architectural decisions)
- Design doc (business logic)
- Code itself (types, doc comments, examples, smaller functions)
- Tests covering the change

> Comments should bother you — re-verify like stale tests.

## 8.5 Replace comments with code

```rust
// ❌
fn save_user(&self) -> Result<(), MyError> {
    // check if authenticated
    if self.is_authenticated() {
        // serialize
        let data = serde_json::to_string(self)?;
        // write to file
        std::fs::write(self.path(), data)?;
    }
}

// ✅
fn save_auth_user(&self) -> Result<PathBuf, MyError> {
    if !self.is_authenticated() { return Err(MyError::UserNotAuthenticated); }
    let path = self.path();
    let serialized = serde_json::to_string(self)?;
    std::fs::write(&path, serialized)?;
    Ok(path)
}
```

## 8.6 TODOs → issues

Don't leave naked `// TODO:`. File issue, link:

```rust
// TODO(issue #42): Remove workaround after bugfix
```

## 8.7 When to `///`

Document all public functions, structs, traits, enums. Cover purpose, usage, behaviors. Add `# Errors`, `# Panics`, `# Safety` sections + plenty examples.

```rust
/// Loads [`User`] from disk.
///
/// # Errors
/// - [`MyError::FileNotFound`] if missing
/// - [`MyError::InvalidJson`] if invalid
fn load_user(path: &Path) -> Result<User, MyError> { … }
```

Examples in doc-tests:
```rust
/// # Examples
/// ```rust
/// assert_eq!(square(4.3), 16)
/// ```
fn square(x: impl ToInt) -> u128 { … }
```

## 8.8 rustdoc lints

| Lint | Description |
|------|-------------|
| `missing_docs` | Public items missing doc |
| `broken_intra_doc_links` | Broken `[`refs`]` (catch on rename) |
| `empty_docs` | Empty doc blocks bypassing `missing_docs` |
| `missing_panics_doc` | Fn can panic but no `# Panics` section |
| `missing_errors_doc` | Returns `Result` but no `# Errors` |
| `missing_safety_doc` | Public unsafe block without `# Safety` |

### `///` vs `//!`

| Style | Used for | Scope |
|---|---|---|
| `///` | Item doc (fn, struct, enum, const) | Public items |
| `//!` | Module/crate doc | At top of `lib.rs` / `mod.rs` |

```rust
//! Custom chess engine.
//! Handles board state, move generation, check detection.
//! # Example
//! ```
//! let board = chess::engine::Board::default();
//! ```
```

## 8.9 Coverage checklist

📦 **Crate (`lib.rs`)**: `//!` explains what crate does + problems solved + crate-level `# Examples`

📁 **Modules**: `//!` explains purpose, exports, invariants. Don't repeat re-exported docs unless clarification needed.

🧱 **Structs/enums/traits**: `///` covers role, invariants, example. Consider `#[non_exhaustive]` if external matches.

🔧 **Functions/methods**: `///` covers what, params, return, edges (`# Panics`/`# Errors`), `# Examples`.

📑 **Traits**: explain purpose (marker? dispatch?). Doc each method (when/why to impl). Document default methods + override conditions.

📦 **Public consts**: what they configure + when used.

### Best practices
- Examples = test cases (free testing)
- Clarity > formality
- `///` for usage; `//` for impl details
- `cargo doc --open` often
- Enforce via `#![deny(missing_docs)]` at top-level
