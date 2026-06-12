---
tags: [skill, hzb, culling, code, pipeline, testing]
---

# Chapter 6 — Generics, Static & Dynamic Dispatch

> Static where you can, dynamic where you must.

Two polymorphism options:
- **Generics / Static dispatch** — compile-time, monomorphized per use
- **Trait objects / Dynamic dispatch** — runtime vtable, single impl

## 6.1 Generics

Abstract stand-ins for concrete types. Compiler **monomorphizes** at compile time → no runtime cost. Generates specialized code per concrete type.

Used for fns, structs, enums, methods, type-state pattern (chapter 7).

## 6.2 Static dispatch — `impl Trait` / `<T: Trait>`

Trait-bounded generic. Compile-time check.

### Best when
- Zero runtime cost (pay compile time)
- Tight loops / perf-critical
- Types known at compile time
- Single-use impl (monomorphized)

```rust
fn specialized_sum<T: MyTrait>(iter: impl Iterator<Item = T>) -> T {
    iter.map(|x| x.random_mapping()).sum()
}
```

Compiles to specialized inlined machine code per call site.

## 6.3 Dynamic dispatch — `dyn Trait`

Used with pointer: `Box<dyn Trait>`, `Arc<dyn Trait>`, `&dyn Trait`.

### Best when
- Runtime polymorphism actually needed
- Heterogeneous impls in one collection
- Abstract internals behind stable interface
- Plugin-style architecture

```rust
trait Animal { fn greet(&self) -> String; }
struct Dog; impl Animal for Dog { fn greet(&self) -> String { "woof".into() } }
struct Cat; impl Animal for Cat { fn greet(&self) -> String { "meow".into() } }

fn all_greet(animals: Vec<Box<dyn Animal>>) {
    for a in animals { println!("{}", a.greet()); }
}
```

## 6.4 Trade-offs

| | Static (`impl Trait`) | Dynamic (`dyn Trait`) |
|---|---|---|
| Performance | ✅ Faster, inlined | ❌ Vtable indirection |
| Compile time | ❌ Slower (monomorphization) | ✅ Faster (shared code) |
| Binary size | ❌ Larger (per-type codegen) | ✅ Smaller |
| Flexibility | ❌ Rigid, one type | ✅ Mix types in collections |
| Trait fns | ❌ Must be object-safe | ✅ Works with trait objects |
| Errors | ✅ Clearer | ❌ Erased types confuse |

- Prefer generics when controlling call site + perf matters
- Use dyn when abstraction/plugins/mixed types needed (runtime cost)
- Unsure → start generic + bound; switch to `Box<dyn>` when flexibility wins

> Favor static dispatch until trait must live behind a pointer.

## 6.5 When dynamic dispatch

### ✅ Use when
- Heterogeneous types in collection
- Runtime plugins / hot-swappable
- Abstract internals from caller (library design)

### ❌ Avoid when
- You control concrete types
- Perf-critical paths
- Same logic expressible via generics

## 6.6 Trait object ergonomics

- Prefer `&dyn Trait` over `Box<dyn Trait>` when no ownership needed
- `Arc<dyn Trait>` for cross-thread shared
- Don't `dyn Trait` if trait has methods returning `Self`
- Avoid premature boxing inside structs:
  ```rust
  // ✅
  struct Renderer<B: Backend> { backend: B }
  // ❌
  struct Renderer { backend: Box<dyn Backend> }
  ```
- If public API must expose `dyn`, box at boundary, not internally

### Object safety

`dyn` requires:
- No generic methods
- No `Self: Sized`
- Methods use `&self` / `&mut self` / `self`

```rust
// ✅ trait Runnable { fn run(&self); }
// ❌ trait Factory { fn create<T>() -> T; }  // generic method
```
