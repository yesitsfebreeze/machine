---
tags: [skill, code, pipeline, process, testing, ui]
---

# Chapter 4 — Error Handling

Rust forces explicit fallibility. Even `unwrap`/`expect` is opt-in.

## 4.1 Prefer `Result`, avoid `panic!`

```rust
fn divide(x: f64, y: f64) -> Result<f64, DivisionError> {
    if y == 0.0 { Err(DivisionError::DividedByZero) } else { Ok(x / y) }
}
```

`panic!` only for unrecoverable: tests, assertions, bugs, explicit crash. Better macros:
- `todo!` — alerts compiler code is missing
- `unreachable!` — reasoned + sure condition impossible; alerts if it ever happens
- `unimplemented!` — block deferred with reason

## 4.2 Avoid `unwrap`/`expect` in prod

`expect` > `unwrap` (has context). Use only in:
- Tests / test helpers
- Failure impossible
- Smart alternatives don't fit

### Alternatives
- Predefined early return → `let Ok(json) = serde_json::from_str(&input) else { return Err(MyError::InvalidJson); }`
- Recovery branch → `if let Ok(json) = … else { … } else { Err(do_something(&input)) }`
- `Option::None` cases → return `Result<T, E>` with module-level error
- `unwrap_or` / `unwrap_or_else` / `unwrap_or_default` for default values

## 4.3 `thiserror` for crate-level errors

```rust
#[derive(Debug, thiserror::Error)]
pub enum MyError {
    #[error("Network Timeout")]
    Timeout,
    #[error("Invalid data: {0}")]
    InvalidData(String),
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
    #[error("Invalid request. Header: {headers}, Metadata: {metadata}")]
    InvalidRequest { headers: Headers, metadata: Metadata }
}
```

### Hierarchies / wrapping

```rust
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("DB error: {0}")]
    Db(#[from] DbError),
    #[error("External services error: {0}")]
    ExternalServices(#[from] ExternalHttpError)
}
```

## 4.4 `anyhow` for binaries only

Ergonomic but erases types. Use in **binaries**, not libraries.

```rust
use anyhow::{Context, Result, anyhow};

fn main() -> Result<()> {
    let content = std::fs::read_to_string("config.json")
        .context("Failed to read config")?;
    Config::from_str(&content)
        .map_err(|err| anyhow!("Config parse: {err}"))
}
```

### Gotchas
- Context strings hard to keep updated codebase-wide vs single thiserror types
- `anyhow::Result` erases type — never in libraries
- Test helpers OK to use anyhow

## 4.5 `?` to bubble

```rust
fn handle_request(req: &Request) -> Result<ValidatedRequest, MyError> {
    validate_headers(req)?;
    validate_body_format(req)?;
    validate_credentials(req)?;
    let body = Body::try_from(req)?;
    Ok(ValidatedRequest::try_from((req, body))?)
}
```

Recovery: `or_else`, `map_err`, `if let Ok(..) else`. Inspect/log: `inspect_err`.

## 4.6 Test errors

Many errors don't impl `PartialEq`. Compare via `to_string()` / `format!`:

```rust
#[test]
fn err_no_partial_eq() {
    let err = divide(10., 0.0).unwrap_err();
    assert_eq!(err.to_string(), "division by zero");
}
```

## 4.7 Other

### Custom error structs (single-error modules)

```rust
#[derive(Debug, thiserror::Error, PartialEq)]
#[error("Request failed with code `{code}`: {message}")]
struct HttpError { code: u16, message: String }
```

### Async errors

In Tokio etc., errors must be `Send + Sync + 'static` across `.await`:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> { Ok(()) }
```

❗ Avoid `Box<dyn std::error::Error>` in libraries.
