---
tags: [skill, code, process, testing, agent, architecture]
---

# Chapter 7 — Type State Pattern

States as types, not runtime flags. Compiler enforces transitions; illegal ops = compile error.

> Invalid states become compile errors instead of runtime bugs.

Available in [Swift](https://swiftology.io/articles/typestate/), [TypeScript](https://catchts.com/type-state) too.

## 7.1 Why

- Avoid runtime state validity checks
- Model state transitions as type transitions (compile-time state machine)
- Prevent misuse (e.g. uninitialized objects)
- Better API safety + DX (only available methods exposed per state)
- `PhantomData` removed at compile — zero memory cost

## 7.2 Simple example: file state

```rust
struct FileNotOpened;
struct FileOpened;

struct File<State> {
    path: PathBuf,
    handle: Option<std::fs::File>,
    _state: std::marker::PhantomData<State>,
}

impl File<FileNotOpened> {
    fn open(path: &Path) -> io::Result<File<FileOpened>> {
        let file = std::fs::File::open(path)?;
        Ok(File {
            path: path.to_path_buf(),
            handle: Some(file),
            _state: std::marker::PhantomData::<FileOpened>,
        })
    }
}

impl File<FileOpened> {
    fn read(&mut self) -> io::Result<String> {
        use io::Read;
        let mut content = String::new();
        let Some(handle) = self.handle.as_mut() else {
            unreachable!("state guarantees handle set");
        };
        handle.read_to_string(&mut content)?;
        Ok(content)
    }
    fn path(&self) -> &PathBuf { &self.path }
}
```

`open` is the only entry — produces `File<FileOpened>` with valid handle. `read` only callable from `FileOpened` state.

## 7.3 Real example: builder with required fields

Forces required fields before `.build()`. Multiple state markers chained:

```rust
struct MissingName; struct NameSet;
struct MissingAge;  struct AgeSet;

struct Builder<NameState, AgeState> {
    name: Option<String>,
    age: u8,
    email: Option<String>,
    _name_marker: PhantomData<NameState>,
    _age_marker: PhantomData<AgeState>,
}

impl Builder<MissingName, MissingAge> {
    fn new() -> Self { … }
    fn name(self, name: String) -> Builder<NameSet, MissingAge> { … }
    fn age(self, age: u8) -> Builder<MissingName, AgeSet> { … }
}

impl Builder<NameSet, MissingAge> {
    fn age(self, age: u8) -> Builder<NameSet, AgeSet> { … }
}

impl Builder<NameSet, AgeSet> {
    fn build(self) -> Person {
        Person { name: self.name.unwrap(), age: self.age, email: self.email }
    }
}
```

```rust
// ✅
Builder::new().name("n".into()).age(30).build();
Builder::new().age(30).name("n".into()).email("e@x".into()).build();

// ❌ compile errors
Builder::new().name("n".into()).build();          // missing age
Builder::new().age(30).build();                   // missing name
Builder::new().build();                           // missing both
```

## 7.4 Real example: network protocol state

Sending pre-connect = won't compile.

```rust
struct Disconnected; struct Connected;

struct Client<State> {
    stream: Option<std::net::TcpStream>,
    _state: std::marker::PhantomData<State>,
}

impl Client<Disconnected> {
    fn connect(addr: &str) -> std::io::Result<Client<Connected>> {
        let stream = std::net::TcpStream::connect(addr)?;
        Ok(Client { stream: Some(stream), _state: PhantomData::<Connected> })
    }
}

impl Client<Connected> {
    fn send(&mut self, msg: &str) {
        use std::io::Write;
        let Some(s) = self.stream.as_mut() else { unreachable!() };
        s.write_all(msg.as_bytes())
    }
}
```

## 7.5 Pros/cons

### Use when
- Compile-time state safety needed
- Enforce API constraints
- Library/crate variant-heavy
- Replace runtime bools/enums with type-safe paths
- Compile-time correctness

### Avoid when
- Trivial states (use enums)
- No type-safety need
- Leads to overcomplicated generics
- Runtime flexibility required

### Downsides
- Verbose
- Complex type signatures
- May need `unsafe` for variant outputs
- Field duplication
- `PhantomData` unintuitive for beginners

> Use when it saves bugs, increases safety, or simplifies logic — not for cleverness.
