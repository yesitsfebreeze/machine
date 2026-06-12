---
tags: [skill, material, code, pipeline, geometry, ui]
---

# Chapter 9 — Pointers

Rust makes memory management explicit. Pointer table:

| Type | Description | `Send + Sync` | Use |
|---|---|---|---|
| `&T` | Shared reference | Yes | Shared read |
| `&mut T` | Exclusive mutable ref | Not Send | Exclusive mutate |
| `Box<T>` | Heap-owned | If T: Send+Sync | Heap alloc |
| `Rc<T>` | Single-thread refcount | Neither | Multi-owner single-thread |
| `Arc<T>` | Atomic refcount | Yes | Multi-owner multi-thread |
| `Cell<T>` | Interior mut for Copy types | Not Sync | Shared mut, non-threaded |
| `RefCell<T>` | Interior mut (runtime borrow check) | Not Sync | Shared mut, non-threaded |
| `Mutex<T>` | Thread-safe interior mut, exclusive | Yes | Shared mut, threaded |
| `RwLock<T>` | Thread-safe shared-read OR exclusive-write | Yes | Shared mut, threaded |
| `OnceCell<T>` | Single-thread one-time init | Not Sync | Lazy value init |
| `LazyCell<T>` | `OnceCell` with closure init | Not Sync | Complex lazy init |
| `OnceLock<T>` | Thread-safe `OnceCell` | Yes | Multi-thread single init |
| `LazyLock<T>` | Thread-safe `LazyCell` | Yes | Multi-thread complex init |
| `*const T` / `*mut T` | Raw pointer | User-managed | FFI / raw memory |

`Send` = data movable across threads. `Sync` = data referenceable from multiple threads.

## When to use

### `&T` — shared borrow
Most common. Safe, no mutation, multiple readers.
```rust
let data = String::from("foo");
print_len(&data); print_capacity(&data);
fn print_len(s: &str) { println!("{}", s.len()); }
```

### `&mut T` — exclusive borrow
Most common mutable. One mutable borrow at a time.
```rust
let mut data = String::from("foo");
mark_update(&mut data);
fn mark_update(s: &mut String) { s.push_str("_update"); }
```

### `Box<T>` — heap-allocated
Single-owner heap. Recursive types, large structs.
```rust
enum BoxedTree<T> {
    Single(T),
    Double(Box<BoxedTree<T>>, Box<BoxedTree<T>>),
}
```

### `Rc<T>` — single-thread refcount
Multiple references in single thread. Linked lists.

### `Arc<T>` — atomic refcount
Multi-thread share. Common: `Arc<[T]>` (read-only), `Arc<Mutex<T>>` (mutable share).

### `RefCell<T>` — runtime borrow check
Shared access + mutation, runtime-enforced. **May panic.**
```rust
let x = RefCell::new(42);
*x.borrow_mut() += 1;
// Don't:
let _b = x.borrow();
let _m = x.borrow_mut();  // PANIC
```

### `Cell<T>` — Copy-only interior mut
Faster, safer than `RefCell`, but only for `Copy`:
```rust
struct S { regular: u8, special: Cell<u8> }
let s = S { regular: 0, special: Cell::new(1) };
s.special.set(100);  // OK even though `s` immutable
```

### `Mutex<T>` — thread-safe mutate
Exclusive access. Usually `Arc<Mutex<T>>` to share.

### `RwLock<T>` — multi-read OR single-write
Like Mutex but multiple readers concurrent. `Arc<RwLock<T>>`.

### `*const T` / `*mut T` — raw
**Unsafe**, FFI only. Explicit.
```rust
let x = 5;
let ptr = &x as *const i32;
unsafe { println!("{}", *ptr) }
```

### `OnceCell` — single-thread one-time init
Share configs between data structures.
```rust
struct MyStruct { distance: usize, root: Option<Rc<OnceCell<MyStruct>>> }
```

### `LazyCell` — `OnceCell` with init closure
Init delayed until first access.

### `OnceLock` — thread-safe `OnceCell`
For static values:
```rust
static CELL: OnceLock<u32> = OnceLock::new();
let val = CELL.get_or_init(|| 12345);
```

### `LazyLock` — thread-safe `LazyCell`
Static values with complex init:
```rust
static CONFIG: LazyLock<HashMap<&str, T>> = LazyLock::new(|| {
    let mut config: HashMap<&str, T> = read_config().into();
    config.insert("special_case", T::default());
    config
});
```

## References
- Mara Bos — *Rust Atomics and Locks*: https://marabos.nl/atomics/
