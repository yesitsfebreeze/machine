# rust-traits

**When**: designing abstractions, choosing static vs dynamic dispatch, encoding invariants.
**Why care**: wrong dispatch tanks perf or destroys ergonomics; missing type-state misses compile-time guarantees.

## Decision tree
- Hot path, monomorphizable → generics (`impl Trait` / `<T: Trait>`). Reason: inlined, no vtable.
- Heterogeneous collections, plugin-like → `dyn Trait`. Reason: uniform size, runtime dispatch.
- State transitions with invalid states → type-state pattern. Reason: invalid transitions become compile errors.
- API stability across crates → sealed trait. Reason: prevents downstream impls breaking on additions.
- Many associated types signal "this is one type, not a trait" — collapse.

## Tradeoffs
- Generics: zero-cost, but explodes binary size and compile time.
- `dyn`: pays vtable lookup; gains binary size, faster compiles.
- Type-state: compile-time safety; pays in API surface (more types).
- Over-trait-ifying everything: indirection without payoff. Concrete first, abstract on second use.

## Anti-patterns (why)
- Trait with one impl: indirection for no benefit. Inline until a second case exists.
- `Box<dyn Trait>` in hot loops: heap + vtable cost where generic would inline.
- Generic over trait that's never extended: complexity tax.
