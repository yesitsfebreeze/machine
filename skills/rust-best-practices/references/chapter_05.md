---
tags: [skill, code, pipeline, testing, agent, spec]
---

# Chapter 5 — Automated Testing

> Tests are the first place people read to understand how code works.

Tests use `#[test]`. Test-only compilation flags: `#[cfg(test)]`.

## 5.1 Tests as living docs

### Descriptive names

Components: `unit_of_work` (function tested) + `expected_behavior` (assertions) + `state` (arrangement).

```rust
// ❌
#[test] fn test_add_happy_path() { assert_eq!(add(2, 2), 4); }

// ✅
#[test]
fn process_should_return_blob_when_larger_than_b() { … }

// ✅ alternative — group in mod
mod process {
    #[test]
    fn should_return_blob_when_larger_than_b() { … }
}
```

### Module organization

Group related tests; IDEs run modules together; module name appears in test output.

```rust
#[cfg(test)]
mod test {
    mod process {
        #[test]
        fn returns_error_xyz_when_b_is_negative() { … }
        #[test]
        fn returns_invalid_input_error_when_a_and_b_not_present() { … }
    }
}
```

### One behavior per test

```rust
// ❌
fn test_thing_parser() {
    assert!(Thing::parse("abcd").is_ok());
    assert!(Thing::parse("ABCD").is_err());
}

// ✅
mod test_thing_parser {
    #[test] fn lowercase_letters_are_valid() {
        assert!(Thing::parse("abcd").is_ok(), "parse err: {:?}", Thing::parse("abcd").unwrap_err());
    }
    #[test] fn capital_letters_are_invalid() { assert!(Thing::parse("ABCD").is_err()); }
}
```

Ok scenarios: include `eprintln`-style msg of `Err` case.

### Few/one assertion per test

Multi-assertion tests obscure intent + require iterations to fix. Use shared setup or `rstest` cases:

```rust
#[rstest]
#[case::single("a")]
#[case::first_letter("ab")]
#[case::last_letter("ba")]
#[case::in_the_middle("bab")]
fn the_function_accepts_all_strings_with_a(#[case] input: &str) {
    assert!(the_function(input).is_ok());
}
```

`rstest` caveats: harder to navigate, expectation/condition naming inverted.

## 5.2 Doc tests

`///` examples become executable tests.
- Run with `cargo test`, NOT `cargo nextest run` (use `cargo t --doc` separately).
- Doc + correctness check, kept fresh by compiler.
- Hide setup with `#`:

```rust
/// # Examples
/// ```rust
/// # use crate_name::generic_add;
/// # assert_eq!(
/// generic_add(5.2, 4) // => 9.2
/// # , 9.2)
/// ```
```

❗ OK to duplicate between doc-tests and unit tests.

## 5.3 Unit / integration / doc

### Unit
Same module as tested unit. Sees private + `pub(crate)`. Focus: implementation + edge cases.
- KISS — one state, one behavior
- Test errors + edges
- Combine under `#[cfg(test)] mod test_unit { … }`
- Minimize external state to public API; focus those on `mod.rs`
- `#[ignore = "msg"]` for unfinished
- `#[should_panic]` for intentional panic

```rust
#[cfg(test)]
mod unit_of_work_tests {
    use super::*;
    #[test]
    fn unit_state_behavior() {
        assert_eq!(result, expected, "Failed: {}", result - expected);
    }
}
```

### Integration
`tests/` directory, external — only public API. Tests interaction between units.
- Happy paths + common cases
- Allow external state ([testcontainers](https://rust.testcontainers.org/) helps)
- Binaries: split `src/main.rs` (executable) + `src/lib.rs` (functions)

```
├── Cargo.toml
├── src/lib.rs
└── tests/
    ├── mod.rs
    ├── common/mod.rs
    └── integration_test.rs
```

### Doc
Happy paths, public API usage, attribute-rich examples.

### Attributes
- `ignore` — skip; for plain text use `text`
- `should_panic` — example expected to panic
- `no_run` — compile, don't execute (side effects)
- `compile_fail` — must fail compile (demo wrong usage)

## 5.4 Asserting

- `assert!(value.is_ok(), "value not Ok: {value:?}")`
- `assert_eq!(result, expected, "diff: {}", result.diff(expected))`

Reminders:
- Use formatted strings — printed on failure
- For pattern match: `assert!(matches!(error, MyError::BadInput(_)), "Expected BadInput, got {error}")`
- `#[should_panic]` only when panic is desired
- Crates: `rstest`, `pretty_assertions` (colorful diffs)

## 5.5 Snapshot testing — `cargo insta`

> When correctness is visual or structural, snapshots beat asserts.

```toml
insta = { version = "1.42.2", features = ["yaml"] }
```

YAML preferred (clean diffs, supports redaction). Install CLI: `cargo install cargo-insta`.

```rust
#[test]
fn test_split_words() {
    let words = split_words("hello from the other side");
    insta::assert_yaml_snapshot!(words);
}
```

Run `cargo insta test` then `cargo insta review`.

### Use for
- Generated code, complex serialized data, rendered HTML, CLI output

### Don't use for
- Stable numeric/small structured data → `assert_eq!`
- Critical path logic → precise unit tests
- Flaky/random output (unless redacted)
- External resources → mocks/stubs

## 5.6 Snapshot best practices

- Name them: `assert_snapshot!("named", output)` → `snapshots/named.snap`
- Keep small + clear:
  ```rust
  // ✅ assert_snapshot!("app_config/http", whole_app_config.http);
  // ❌ assert_snapshot!("app_config", whole_app_config); // huge
  ```
- Don't snapshot primitives → `assert_eq!`
- Redact unstable fields:
  ```rust
  assert_json_snapshot!("get_user_data", data,
      ".created_at" => "[timestamp]",
      ".id" => "[uuid]");
  ```
- Commit snapshots to git
- Review carefully before accepting
