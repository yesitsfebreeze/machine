# rust-ownership

**When**: choosing between borrow, clone, `Arc`, `Rc`, `Cow`, or move semantics.
**Why care**: wrong choice cascades — extra clones hurt perf, wrong `Arc` placement creates contention, missing borrow forces API rewrites.

## Decision tree
- Single owner, no sharing → move. Reason: simplest, zero overhead.
- Caller keeps value, callee reads briefly → `&T`. Reason: no allocation, no refcount.
- Multiple readers, single thread → `Rc<T>`. Reason: refcount cheaper than atomic.
- Multiple readers across threads → `Arc<T>`. Reason: atomic refcount needed for `Send + Sync`.
- Cheap to clone, callee may need owned → `Cow<T>`. Reason: defer the choice to runtime.
- "Just clone it" is fine if T is small/Copy or hot path proves clone is noise.

## Tradeoffs
- Clone-heavy code reads simpler but obscures perf cost. Prefer borrows in library APIs; allow clones in app code where data is small.
- `Arc<Mutex<T>>` is a smell when ownership could be redesigned with channels or actor-style isolation.
- Reaching for `Rc<RefCell<T>>` often means the data model wants a graph — consider arena/index-based instead.

## Anti-patterns (why)
- `Arc<String>` where `Arc<str>` suffices: extra indirection without benefit.
- Cloning to satisfy borrow checker repeatedly: signals lifetime model is wrong, not that clones are needed.
