//! Time source abstraction.
//!
//! The daemon derives liveness, lease expiry, and message TTLs from wall-clock
//! time. Tests need to advance time deterministically (e.g. to fire a lease
//! expiry without sleeping), so all time reads go through a [`Clock`] handle
//! rather than calling [`OffsetDateTime::now_utc`] directly.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use time::OffsetDateTime;

use crate::error::Result;

/// A monotonic-enough wall clock. Either follows the system clock or, in tests,
/// a manually advanced fixed clock.
#[derive(Clone)]
pub struct Clock {
    inner: ClockKind,
}

#[derive(Clone)]
enum ClockKind {
    System,
    /// Fixed clock holding Unix-epoch seconds; advance via [`Clock::advance`].
    Fixed(Arc<AtomicI64>),
}

impl Clock {
    /// System wall clock (production).
    pub fn system() -> Self {
        Clock {
            inner: ClockKind::System,
        }
    }

    /// A controllable clock anchored at `unix_seconds` (tests).
    pub fn fixed_at(unix_seconds: i64) -> Self {
        Clock {
            inner: ClockKind::Fixed(Arc::new(AtomicI64::new(unix_seconds))),
        }
    }

    /// Current time in Unix-epoch seconds.
    pub fn now_unix(&self) -> i64 {
        match &self.inner {
            ClockKind::System => OffsetDateTime::now_utc().unix_timestamp(),
            ClockKind::Fixed(v) => v.load(Ordering::SeqCst),
        }
    }

    /// Push a fixed clock forward by `seconds`. No-op on the system clock.
    pub fn advance(&self, seconds: i64) {
        if let ClockKind::Fixed(v) = &self.inner {
            v.fetch_add(seconds, Ordering::SeqCst);
        }
    }

    /// Current time as an ISO-8601 UTC string (the wire format for all `ts` fields).
    pub fn now_iso(&self) -> Result<String> {
        iso_from_unix(self.now_unix())
    }
}

/// Render a Unix-epoch-seconds value as ISO-8601 UTC, e.g. `2026-06-14T12:00:00Z`.
pub fn iso_from_unix(unix_seconds: i64) -> Result<String> {
    let dt = OffsetDateTime::from_unix_timestamp(unix_seconds)
        .unwrap_or(OffsetDateTime::UNIX_EPOCH);
    Ok(dt.format(&time::format_description::well_known::Rfc3339)?)
}
