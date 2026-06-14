//! Wire contract: request and response shapes for the eight verbs (SPEC §4).
//!
//! These are serde types. The MCP layer deserializes tool arguments into the
//! `*Request` types and serializes the `*Response` types back. Field names match
//! the SPEC tables exactly; optional fields use `Option` and are omitted when
//! `None`.

use serde::{Deserialize, Serialize};

// --- shared enums ---------------------------------------------------------

/// Claim acquisition mode (SPEC §4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimMode {
    #[default]
    Exclusive,
    Shared,
}

/// Wait policy when a resource is already held (SPEC §4.2, C-7/C-9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaitPolicy {
    #[default]
    NoWait,
    Queue,
}

/// Outcome of a `claim` request (SPEC §4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    Granted,
    Queued,
    Denied,
}

/// Outcome of a `release` request (SPEC §4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseStatus {
    Released,
    NotHolder,
    Unknown,
}

/// Derived liveness of a roster entry (SPEC §6, R-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Liveness {
    Alive,
    Stale,
    Dead,
}

// --- awareness: register --------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
    pub agent_id: String,
    pub branch: String,
    pub prompt_ptr: String,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub ttl_seconds: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegisterResponse {
    pub agent_id: String,
    pub registered_at: String,
    pub expires_at: String,
    pub epoch: i64,
}

// --- awareness: roster ----------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct RosterRequest {
    pub agent_id: String,
    #[serde(default)]
    pub include_stale: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RosterEntry {
    pub agent_id: String,
    pub branch: String,
    pub prompt_ptr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub liveness: Liveness,
    pub last_seen: String,
    pub expires_at: String,
    pub held_claims: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RosterResponse {
    pub agents: Vec<RosterEntry>,
}

// --- claims: claim --------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct ClaimRequest {
    pub agent_id: String,
    pub resource: String,
    #[serde(default)]
    pub mode: ClaimMode,
    #[serde(default)]
    pub lease_seconds: Option<i64>,
    #[serde(default)]
    pub wait: WaitPolicy,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClaimResponse {
    pub status: ClaimStatus,
    pub resource: String,
    pub claim_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lease_expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_position: Option<i64>,
    pub fence: i64,
}

// --- claims: release ------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseRequest {
    pub agent_id: String,
    pub claim_id: String,
    pub resource: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReleaseResponse {
    pub status: ReleaseStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_holder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fence: Option<i64>,
}

// --- claims: claims (inspect) ---------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct ClaimsRequest {
    pub agent_id: String,
    #[serde(default)]
    pub resource: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueueTicket {
    pub agent_id: String,
    pub claim_id: String,
    pub position: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClaimView {
    pub resource: String,
    pub mode: ClaimMode,
    /// One holder for `exclusive`; a list for `shared` (serialized as an array
    /// in both cases for a stable shape).
    pub holder: Vec<String>,
    pub claim_id: String,
    pub fence: i64,
    pub lease_expires_at: String,
    pub queue: Vec<QueueTicket>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClaimsResponse {
    pub claims: Vec<ClaimView>,
}

// --- messaging: post ------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct PostRequest {
    pub agent_id: String,
    pub to: String,
    #[serde(default)]
    pub subject: Option<String>,
    pub body: String,
    #[serde(default)]
    pub reply_to: Option<String>,
    #[serde(default)]
    pub ttl_seconds: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostResponse {
    pub message_id: String,
    pub posted_at: String,
    pub fanout: i64,
}

// --- messaging: inbox -----------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct InboxRequest {
    pub agent_id: String,
    #[serde(default)]
    pub since: Option<String>,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InboxMessage {
    pub message_id: String,
    pub from: String,
    pub to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    pub body: String,
    pub posted_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InboxResponse {
    pub messages: Vec<InboxMessage>,
    pub cursor: String,
    pub unread: i64,
}

// --- messaging: read ------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct ReadRequest {
    pub agent_id: String,
    pub up_to: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadResponse {
    pub cursor: String,
    pub remaining: i64,
}
