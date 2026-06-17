//! Monotonic ULID generation.
//!
//! ULIDs are Crockford base32, uppercase, 26 chars. The `ulid` crate's
//! `Generator` yields monotonically increasing ULIDs within a process even at
//! sub-millisecond rates, so the message cursor total-order never ties.

use std::sync::Mutex;
use ulid::Generator;

/// Process-wide monotonic ULID generator.
pub struct UlidGen {
    gen: Mutex<Generator>,
}

impl UlidGen {
    pub fn new() -> Self {
        UlidGen {
            gen: Mutex::new(Generator::new()),
        }
    }

    /// Produce the next ULID as a 26-char uppercase Crockford base32 string.
    /// On the rare monotonic overflow within a single millisecond, fall back to
    /// a fresh (non-monotonic but still time-ordered) ULID.
    pub fn next(&self) -> String {
        let mut g = self.gen.lock().unwrap();
        match g.generate() {
            Ok(u) => u.to_string(),
            Err(_) => ulid::Ulid::new().to_string(),
        }
    }
}

impl Default for UlidGen {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ulid_is_26_uppercase_chars() {
        let g = UlidGen::new();
        let u = g.next();
        assert_eq!(u.len(), 26, "ULID must be 26 chars");
        assert!(
            u.chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()),
            "ULID must be uppercase Crockford base32: {u}"
        );
    }

    #[test]
    fn ulids_are_monotonic() {
        let g = UlidGen::new();
        let mut prev = g.next();
        for _ in 0..1000 {
            let cur = g.next();
            assert!(cur > prev, "ULIDs must strictly increase: {prev} >= {cur}");
            prev = cur;
        }
    }
}
