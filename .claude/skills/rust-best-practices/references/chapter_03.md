---
tags: [skill, material, code, pipeline, perf, testing]
---

# Chapter 3 — Performance Mindset

> Don't guess, measure.

Rust is fast. Don't optimize without evidence.

## First steps

- Build with `--release` (#1 cause of "Rust slow" complaints = forgot release flag)
- `cargo clippy -- -D clippy::perf`
- `cargo bench` — micro-benchmarks; bench against original; >5% improvement = worth it
- `cargo flamegraph` (Linux); `samply` better DX on macOS

## 3.1 Flamegraph

```bash
cargo install flamegraph
cargo flamegraph                          # defaults to release
cargo flamegraph --bin=stress2
cargo flamegraph --unit-test crate -- test::path
cargo flamegraph --test test_name         # integration tests
cargo flamegraph --bench bench_name --features f -- --bench
```

❗ Always profile with `--release`. `--dev` lacks optimizations and is unrealistic.

Reading:
- y-axis = stack depth (main near bottom, called fns above)
- box width = total CPU time (wider = more cycles or more calls)
- color = random, not significant

Thick stacks = heavy CPU. Thin = cheap.

## 3.2 Avoid redundant cloning

Cloning is cheap... until it isn't.

### When ownership IS warranted
- API requires owned data
- Overloaded `std::ops` but need original (e.g. `Add for Point` consumes self)
- Comparison snapshots (clone before mutation, compare after)
- `Arc`/`Rc` ref counters
- Builder requires owned mutation
- Model business state (e.g. `Validate::try_from(raw)` consumes raw to mark validated)

### When NOT
- Prefer ref-taking APIs (`fn process(values: &[T])`) over owned (`Vec<T>`)
- Read-only iteration: `.iter()` / slices
- Mutate from another thread → `&mut MyStruct`

### `Cow` for "maybe owned"

```rust
use std::borrow::Cow;
fn hello_greet(name: Cow<'_, str>) { println!("Hello {name}"); }
hello_greet(Cow::Borrowed("Julia"));
hello_greet(Cow::Owned("Naomi".to_string()));
```

## 3.3 Stack vs heap

### Good
- Small types (`Copy`, `usize`, `bool`) on stack
- Pass huge types (`>512B`) by reference, not value
- Heap-allocate recursive structures:
  ```rust
  enum OctreeNode<T> { Node(T), Children(Box<[Node<T>; 8]>) }
  ```
- Return small types by value (Copy or cheap-clone)

### Mind
- `#[inline]` only when bench proves benefit — Rust inlines well already
- Avoid massive stack alloc; box: `Box<[u8; 65536]>` allocs on stack first then boxes. Better: `vec![0; 65536].into_boxed_slice()`
- Large const arrays → `smallvec` crate

## 3.4 Iterators = zero-cost

Lazy, compiled to tight loops. Chained `.filter().map().rev().skip().take().collect()` doesn't cost extra.

- Prefer iterators over manual `for` on collections
- `.iter()` creates reference — multiple iterators share original

❗ Avoid intermediate collections:

```rust
// ❌
let doubled: Vec<_> = items.iter().map(|x| x * 2).collect();
process(doubled);

// ✅ — process accepts impl Iterator
let doubled_iter = items.iter().map(|x| x * 2);
process(doubled_iter);
```
