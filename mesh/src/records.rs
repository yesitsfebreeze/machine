//! Internal persisted records (LMDB JSON blobs). These are distinct from the
//! wire `types` so storage representation can evolve independently of the
//! contract.

use serde::{Deserialize, Serialize};

use crate::types::ClaimMode;

/// One roster entry, keyed by `agent_id` in the roster DB (SPEC §6, R-2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RosterRecord {
    pub agent_id: String,
    pub branch: String,
    pub prompt_ptr: String,
    pub role: Option<String>,
    /// First-seen Unix seconds; stable across heartbeats.
    pub registered_at: i64,
    /// Last heartbeat, Unix seconds.
    pub last_seen: i64,
    /// `last_seen + ttl`, Unix seconds.
    pub expires_at: i64,
    pub ttl_seconds: i64,
    /// Incremented each fresh join after death, so peers can detect a restart.
    pub epoch: i64,
}

/// A queued waiter for a held resource (SPEC C-7/C-8).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedTicket {
    pub agent_id: String,
    pub claim_id: String,
    pub mode: ClaimMode,
    pub lease_seconds: i64,
    pub note: Option<String>,
    /// Enqueue time (Unix seconds) — preserves FIFO promotion order.
    pub enqueued_at: i64,
}

/// A single live holder of a resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Holder {
    pub agent_id: String,
    pub claim_id: String,
    /// Auto-expiry of this hold, Unix seconds (SPEC C-12).
    pub lease_expires_at: i64,
    pub lease_seconds: i64,
    pub note: Option<String>,
}

/// The live claim state for one resource, keyed by `resource` (SPEC §5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimRecord {
    pub resource: String,
    pub mode: ClaimMode,
    /// One holder for `exclusive`; one-or-more for `shared`.
    pub holders: Vec<Holder>,
    /// FIFO queue of waiters.
    pub queue: Vec<QueuedTicket>,
    /// Monotonic fence; rises on every fresh grant of this resource (SPEC C-4).
    pub fence: i64,
}

impl ClaimRecord {
    /// True when no holders remain (the resource is free).
    pub fn is_free(&self) -> bool {
        self.holders.is_empty()
    }
}

/// A stored message body, keyed by `message_id` (SPEC §9). Stored once even for
/// broadcast/topic; recipients read through their own cursor (M-8).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRecord {
    pub message_id: String,
    pub from: String,
    /// Original addressing: an `agent_id`, `*`, or `topic:<name>`.
    pub to: String,
    pub subject: Option<String>,
    pub body: String,
    pub reply_to: Option<String>,
    /// Server post time, Unix seconds.
    pub posted_at: i64,
    /// Absolute expiry (Unix seconds) when a TTL was given (SPEC M-6).
    pub expires_at: Option<i64>,
}
