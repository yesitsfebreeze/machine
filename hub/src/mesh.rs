//! The hub daemon core: the eight coordination verbs with full behavioral
//! parity to the prior `mesh/mesh.mjs`. State lives in `.mesh/` (unchanged for
//! data continuity). An injectable clock makes lease/liveness deterministic in
//! tests.

use crate::error::{HubError, Result};
use crate::state::{ClaimRecord, Event, Holder, LogEntry, Message, RosterEntry, State, Store, Ticket};
use crate::ulid_gen::UlidGen;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;

pub const DEFAULT_TTL_SECONDS: i64 = 60;
pub const STALE_GRACE_SECONDS: i64 = 30;
pub const DEFAULT_LEASE_SECONDS: i64 = 120;
pub const ZERO_CURSOR: &str = "00000000000000000000000000";

/// A clock returning Unix seconds. Injectable for deterministic tests.
pub type Clock = Arc<dyn Fn() -> i64 + Send + Sync>;

fn system_clock() -> Clock {
    Arc::new(|| {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    })
}

/// RFC3339 UTC at second precision, e.g. `2026-06-14T12:00:00Z` (no millis).
pub fn iso_from_unix(unix_seconds: i64) -> String {
    // Days-from-civil algorithm (Howard Hinnant). Pure integer math, no deps.
    let secs = unix_seconds;
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let (hour, min, sec) = (rem / 3600, (rem % 3600) / 60, rem % 60);

    // z = days since 1970-01-01; shift to era based on 0000-03-01.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let day = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let month = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let year = if month <= 2 { y + 1 } else { y };

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, min, sec
    )
}

/// The hub daemon. Holds the on-disk store, the clock, and the ULID generator.
pub struct Mesh {
    store: Store,
    now_fn: Clock,
    ulid: UlidGen,
}

impl Mesh {
    /// Open the daemon over `dir` (the `.mesh` data dir) with the system clock.
    pub fn open(dir: impl AsRef<Path>) -> Result<Self> {
        Ok(Mesh {
            store: Store::new(dir)?,
            now_fn: system_clock(),
            ulid: UlidGen::new(),
        })
    }

    /// Open with an injected clock (tests).
    #[cfg(test)]
    pub fn open_with_clock(dir: impl AsRef<Path>, clock: Clock) -> Result<Self> {
        Ok(Mesh {
            store: Store::new(dir)?,
            now_fn: clock,
            ulid: UlidGen::new(),
        })
    }

    fn now(&self) -> i64 {
        (self.now_fn)()
    }

    // --- awareness: register / roster ---------------------------------------

    pub fn register(&self, req: &Value) -> Result<Value> {
        require_fields(req, &["agent_id", "branch", "prompt_ptr"])?;
        let now = self.now();
        let ttl = std::cmp::max(1, opt_i64(req, "ttl_seconds").unwrap_or(DEFAULT_TTL_SECONDS));
        let agent_id = str_field(req, "agent_id");
        let branch = str_field(req, "branch");
        let prompt_ptr = str_field(req, "prompt_ptr");
        let role = opt_str(req, "role");
        let ulid = &self.ulid;
        let _ = ulid;

        self.store.txn(|state| {
            let (mut registered_at, mut epoch) = (now, 1i64);
            if let Some(prev) = state.roster.get(&agent_id) {
                registered_at = prev.registered_at;
                let was_dead = liveness_at(prev.expires_at, now) == "dead";
                epoch = if was_dead { prev.epoch + 1 } else { prev.epoch };
            }
            let rec = RosterEntry {
                agent_id: agent_id.clone(),
                branch: branch.clone(),
                prompt_ptr: prompt_ptr.clone(),
                role: role.clone(),
                registered_at,
                last_seen: now,
                expires_at: now + ttl,
                epoch,
            };
            let value = json!({
                "agent_id": rec.agent_id,
                "registered_at": iso_from_unix(rec.registered_at),
                "expires_at": iso_from_unix(rec.expires_at),
                "epoch": rec.epoch,
            });
            state.roster.insert(agent_id.clone(), rec);
            Ok((true, value))
        })
    }

    pub fn roster(&self, req: &Value) -> Result<Value> {
        require_fields(req, &["agent_id"])?;
        let now = self.now();
        let include_stale = req.get("include_stale").and_then(|v| v.as_bool()).unwrap_or(false);

        self.store.txn(|state| {
            let changed = sweep(state, now);
            let held = held_claims_index(state, now);
            let mut agents: Vec<Value> = Vec::new();
            // Deterministic order: BTreeMap iterates by agent_id already.
            for rec in state.roster.values() {
                let liveness = liveness_at(rec.expires_at, now);
                if liveness == "dead" && !include_stale {
                    continue;
                }
                let mut claims: Vec<String> =
                    held.get(&rec.agent_id).cloned().unwrap_or_default();
                claims.sort();
                let mut entry = json!({
                    "agent_id": rec.agent_id,
                    "branch": rec.branch,
                    "prompt_ptr": rec.prompt_ptr,
                    "liveness": liveness,
                    "last_seen": iso_from_unix(rec.last_seen),
                    "expires_at": iso_from_unix(rec.expires_at),
                    "held_claims": claims,
                });
                if let Some(role) = &rec.role {
                    entry["role"] = json!(role);
                }
                agents.push(entry);
            }
            agents.sort_by(|a, b| {
                a["agent_id"].as_str().cmp(&b["agent_id"].as_str())
            });
            Ok((changed, json!({ "agents": agents })))
        })
    }

    // --- claims: claim / release / claims -----------------------------------

    pub fn claim(&self, req: &Value) -> Result<Value> {
        let agent_id = opt_str(req, "agent_id").unwrap_or_default();
        let resource = opt_str(req, "resource").unwrap_or_default();
        if agent_id.is_empty() || resource.is_empty() {
            return Err(HubError::new("agent_id and resource are required"));
        }
        let now = self.now();
        let mode = opt_str(req, "mode").unwrap_or_else(|| "exclusive".to_string());
        let lease = std::cmp::max(1, opt_i64(req, "lease_seconds").unwrap_or(DEFAULT_LEASE_SECONDS));
        let wait = opt_str(req, "wait").unwrap_or_else(|| "no_wait".to_string());
        let note = opt_str(req, "note");

        self.store.txn(|state| {
            let mut rec = state.claims.remove(&resource).unwrap_or_else(|| ClaimRecord {
                resource: resource.clone(),
                mode: mode.clone(),
                holders: Vec::new(),
                queue: Vec::new(),
                fence: *state.fence_floor.get(&resource).unwrap_or(&0),
            });
            reap_record(&state.roster, &mut rec, now);

            // Idempotent renewal by the current holder.
            if let Some(holder) = rec.holders.iter_mut().find(|h| h.agent_id == agent_id) {
                holder.lease_expires_at = now + lease;
                if note.is_some() {
                    holder.note = note.clone();
                }
                let value = json!({
                    "status": "granted",
                    "resource": resource,
                    "claim_id": holder.claim_id,
                    "holder": agent_id,
                    "lease_expires_at": iso_from_unix(holder.lease_expires_at),
                    "fence": rec.fence,
                });
                state.claims.insert(resource.clone(), rec);
                return Ok((true, value));
            }

            if grantable(&rec, &mode) {
                let claim_id = self.ulid.next();
                rec.mode = mode.clone();
                rec.fence += 1;
                rec.holders.push(Holder {
                    agent_id: agent_id.clone(),
                    claim_id: claim_id.clone(),
                    lease_expires_at: now + lease,
                    note: note.clone(),
                });
                let fence = rec.fence;
                state.events.push(Event {
                    resource: resource.clone(),
                    event: "grant".to_string(),
                    agent_id: agent_id.clone(),
                    fence,
                    at: iso_from_unix(now),
                });
                let value = json!({
                    "status": "granted",
                    "resource": resource,
                    "claim_id": claim_id,
                    "holder": agent_id,
                    "lease_expires_at": iso_from_unix(now + lease),
                    "fence": fence,
                });
                state.claims.insert(resource.clone(), rec);
                return Ok((true, value));
            }

            // Held by someone else: queue or deny.
            let current_holder = rec.holders.first().map(|h| h.agent_id.clone());
            if wait == "queue" {
                let claim_id = self.ulid.next();
                rec.queue.push(Ticket {
                    agent_id: agent_id.clone(),
                    claim_id: claim_id.clone(),
                    mode: mode.clone(),
                    lease_seconds: lease,
                    note: note.clone(),
                });
                let queue_position = rec.queue.len() as i64;
                let fence = rec.fence;
                let mut value = json!({
                    "status": "queued",
                    "resource": resource,
                    "claim_id": claim_id,
                    "queue_position": queue_position,
                    "fence": fence,
                });
                if let Some(h) = &current_holder {
                    value["holder"] = json!(h);
                }
                state.claims.insert(resource.clone(), rec);
                return Ok((true, value));
            }
            // no_wait: persist any reaping, grant nothing.
            let fence = rec.fence;
            let mut value = json!({
                "status": "denied",
                "resource": resource,
                "claim_id": "",
                "fence": fence,
            });
            if let Some(h) = &current_holder {
                value["holder"] = json!(h);
            }
            state.claims.insert(resource.clone(), rec);
            Ok((true, value))
        })
    }

    pub fn release(&self, req: &Value) -> Result<Value> {
        require_fields(req, &["agent_id", "claim_id", "resource"])?;
        let now = self.now();
        let agent_id = str_field(req, "agent_id");
        let claim_id = str_field(req, "claim_id");
        let resource = str_field(req, "resource");

        self.store.txn(|state| {
            let mut rec = match state.claims.remove(&resource) {
                Some(r) => r,
                None => return Ok((false, json!({ "status": "unknown" }))),
            };

            let hi = rec
                .holders
                .iter()
                .position(|h| h.agent_id == agent_id && h.claim_id == claim_id);
            let qi = rec
                .queue
                .iter()
                .position(|t| t.agent_id == agent_id && t.claim_id == claim_id);

            if hi.is_none() && qi.is_none() {
                let known = rec.holders.iter().any(|h| h.claim_id == claim_id)
                    || rec.queue.iter().any(|t| t.claim_id == claim_id);
                let status = if known { "not_holder" } else { "unknown" };
                state.claims.insert(resource.clone(), rec);
                return Ok((false, json!({ "status": status })));
            }

            if let Some(qi) = qi {
                rec.queue.remove(qi); // cancel queued ticket, no promotion
                let fence = rec.fence;
                state.claims.insert(resource.clone(), rec);
                return Ok((true, json!({ "status": "released", "fence": fence })));
            }

            rec.holders.remove(hi.unwrap());
            state.events.push(Event {
                resource: resource.clone(),
                event: "release".to_string(),
                agent_id: agent_id.clone(),
                fence: rec.fence,
                at: iso_from_unix(now),
            });
            let promoted = promote_queue(&mut rec, now);
            if let Some(p) = &promoted {
                state.events.push(Event {
                    resource: resource.clone(),
                    event: "promote".to_string(),
                    agent_id: p.clone(),
                    fence: rec.fence,
                    at: iso_from_unix(now),
                });
            }
            let fence = rec.fence;
            persist_or_delete(state, &resource, rec);
            let mut value = json!({ "status": "released", "fence": fence });
            if let Some(p) = promoted {
                value["next_holder"] = json!(p);
            }
            Ok((true, value))
        })
    }

    pub fn claims(&self, req: &Value) -> Result<Value> {
        require_fields(req, &["agent_id"])?;
        let now = self.now();
        let filter = opt_str(req, "resource");

        self.store.txn(|state| {
            let changed = sweep(state, now);
            let mut out: Vec<Value> = Vec::new();
            for rec in state.claims.values() {
                if rec.holders.is_empty() {
                    continue;
                }
                if let Some(f) = &filter {
                    if &rec.resource != f {
                        continue;
                    }
                }
                out.push(claim_view(rec));
            }
            out.sort_by(|a, b| a["resource"].as_str().cmp(&b["resource"].as_str()));
            Ok((changed, json!({ "claims": out })))
        })
    }

    // --- messaging: post / inbox / read -------------------------------------

    pub fn post(&self, req: &Value) -> Result<Value> {
        let agent_id = opt_str(req, "agent_id").unwrap_or_default();
        let to = opt_str(req, "to").unwrap_or_default();
        let body = opt_str(req, "body").unwrap_or_default();
        if agent_id.is_empty() || to.is_empty() || body.is_empty() {
            return Err(HubError::new("agent_id, to, and body are required"));
        }
        let now = self.now();
        let message_id = self.ulid.next();
        let subject = opt_str(req, "subject");
        let reply_to = opt_str(req, "reply_to");
        let expires_unix = opt_i64(req, "ttl_seconds").map(|t| now + std::cmp::max(1, t));

        self.store.txn(|state| {
            state.messages.insert(
                message_id.clone(),
                Message {
                    message_id: message_id.clone(),
                    from: agent_id.clone(),
                    to: to.clone(),
                    subject: subject.clone(),
                    body: body.clone(),
                    reply_to: reply_to.clone(),
                    posted_at: now,
                    expires_at: expires_unix,
                },
            );
            state.log.push(LogEntry {
                message_id: message_id.clone(),
                sender: agent_id.clone(),
                recipient: to.clone(),
                posted_at: iso_from_unix(now),
                expires_unix,
            });
            let fanout = if to == "*" {
                state.roster.len() as i64
            } else if to.starts_with("topic:") {
                0
            } else {
                1
            };
            Ok((
                true,
                json!({
                    "message_id": message_id,
                    "posted_at": iso_from_unix(now),
                    "fanout": fanout,
                }),
            ))
        })
    }

    pub fn inbox(&self, req: &Value) -> Result<Value> {
        require_fields(req, &["agent_id"])?;
        let now = self.now();
        let agent_id = str_field(req, "agent_id");
        let limit = clamp(opt_i64(req, "limit").unwrap_or(100), 1, 1000) as usize;
        let topics = str_array(req, "topics");
        let since = opt_str(req, "since");

        self.store.txn(|state| {
            let live_cursor = state
                .cursors
                .get(&agent_id)
                .cloned()
                .unwrap_or_else(|| ZERO_CURSOR.to_string());
            let cursor = since.clone().unwrap_or_else(|| live_cursor.clone());
            let pending = pending_after(state, &agent_id, &cursor, &topics, now);
            let mut messages: Vec<Value> = Vec::new();
            for id in pending.iter().take(limit) {
                if let Some(m) = state.messages.get(id) {
                    messages.push(to_inbox_message(m));
                }
            }
            let unread = std::cmp::max(0, pending.len() as i64 - messages.len() as i64);
            Ok((
                false,
                json!({
                    "messages": messages,
                    "cursor": live_cursor,
                    "unread": unread,
                }),
            ))
        })
    }

    pub fn read(&self, req: &Value) -> Result<Value> {
        require_fields(req, &["agent_id", "up_to"])?;
        let now = self.now();
        let agent_id = str_field(req, "agent_id");
        let up_to = str_field(req, "up_to");

        self.store.txn(|state| {
            let current = state
                .cursors
                .get(&agent_id)
                .cloned()
                .unwrap_or_else(|| ZERO_CURSOR.to_string());
            let new_cursor = if up_to > current { up_to.clone() } else { current };
            state.cursors.insert(agent_id.clone(), new_cursor.clone());
            let remaining = pending_after(state, &agent_id, &new_cursor, &[], now).len() as i64;
            Ok((
                true,
                json!({ "cursor": new_cursor, "remaining": remaining }),
            ))
        })
    }

    // --- maintenance: gc ----------------------------------------------------

    /// Drop TTL-expired messages and sweep dead claims. Returns reclaimed count.
    pub fn gc(&self) -> Result<i64> {
        let now = self.now();
        self.store.txn(|state| {
            sweep(state, now);
            let mut reclaimed = 0i64;
            let mut kept: Vec<LogEntry> = Vec::with_capacity(state.log.len());
            for row in std::mem::take(&mut state.log) {
                let expired = row.expires_unix.map(|e| e <= now).unwrap_or(false);
                if expired {
                    state.messages.remove(&row.message_id);
                    reclaimed += 1;
                } else {
                    kept.push(row);
                }
            }
            state.log = kept;
            Ok((true, reclaimed))
        })
    }
}

// --- pure helpers ----------------------------------------------------------

fn require_fields(req: &Value, fields: &[&str]) -> Result<()> {
    for f in fields {
        let missing = match req.get(*f) {
            None | Some(Value::Null) => true,
            Some(Value::String(s)) => s.is_empty(),
            _ => false,
        };
        if missing {
            return Err(HubError::new(format!("{} are required", fields.join(", "))));
        }
    }
    Ok(())
}

fn str_field(req: &Value, key: &str) -> String {
    req.get(key).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

fn opt_str(req: &Value, key: &str) -> Option<String> {
    match req.get(key) {
        Some(Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

fn opt_i64(req: &Value, key: &str) -> Option<i64> {
    req.get(key).and_then(|v| v.as_i64())
}

fn str_array(req: &Value, key: &str) -> Vec<String> {
    match req.get(key) {
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        _ => Vec::new(),
    }
}

fn clamp(n: i64, lo: i64, hi: i64) -> i64 {
    std::cmp::min(hi, std::cmp::max(lo, n))
}

fn liveness_at(expires_at: i64, now: i64) -> &'static str {
    if now <= expires_at {
        "alive"
    } else if now <= expires_at + STALE_GRACE_SECONDS {
        "stale"
    } else {
        "dead"
    }
}

fn agent_is_dead(roster: &std::collections::BTreeMap<String, RosterEntry>, agent_id: &str, now: i64) -> bool {
    match roster.get(agent_id) {
        Some(rec) => liveness_at(rec.expires_at, now) == "dead",
        None => false,
    }
}

fn grantable(rec: &ClaimRecord, mode: &str) -> bool {
    if rec.holders.is_empty() {
        return true;
    }
    rec.mode == "shared" && mode == "shared"
}

fn holder_from_ticket(ticket: Ticket, now: i64) -> Holder {
    Holder {
        agent_id: ticket.agent_id,
        claim_id: ticket.claim_id,
        lease_expires_at: now + ticket.lease_seconds,
        note: ticket.note,
    }
}

fn promote_queue(rec: &mut ClaimRecord, now: i64) -> Option<String> {
    if !rec.holders.is_empty() || rec.queue.is_empty() {
        return None;
    }
    let ticket = rec.queue.remove(0);
    rec.mode = ticket.mode.clone();
    rec.fence += 1;
    let first = ticket.agent_id.clone();
    rec.holders.push(holder_from_ticket(ticket, now));
    if rec.mode == "shared" {
        while rec.queue.first().map(|t| t.mode == "shared").unwrap_or(false) {
            let t = rec.queue.remove(0);
            rec.holders.push(holder_from_ticket(t, now));
        }
    }
    Some(first)
}

/// Reap one record in place: drop expired/dead holders and dead-agent queue
/// tickets, then promote a waiter if the resource went free. Returns whether it
/// changed.
fn reap_record(
    roster: &std::collections::BTreeMap<String, RosterEntry>,
    rec: &mut ClaimRecord,
    now: i64,
) -> bool {
    let before_h = rec.holders.len();
    let before_q = rec.queue.len();
    rec.holders
        .retain(|h| h.lease_expires_at > now && !agent_is_dead(roster, &h.agent_id, now));
    rec.queue.retain(|t| !agent_is_dead(roster, &t.agent_id, now));
    let mut changed = rec.holders.len() != before_h || rec.queue.len() != before_q;
    if rec.holders.is_empty() && !rec.queue.is_empty() && promote_queue(rec, now).is_some() {
        changed = true;
    }
    changed
}

/// Sweep every claim record; delete fully idle ones (stamping the fence floor).
fn sweep(state: &mut State, now: i64) -> bool {
    let mut changed = false;
    let resources: Vec<String> = state.claims.keys().cloned().collect();
    for resource in resources {
        let mut rec = state.claims.remove(&resource).unwrap();
        if reap_record(&state.roster, &mut rec, now) {
            changed = true;
            persist_or_delete(state, &resource, rec);
        } else {
            state.claims.insert(resource, rec);
        }
    }
    changed
}

fn persist_or_delete(state: &mut State, resource: &str, rec: ClaimRecord) {
    if rec.holders.is_empty() && rec.queue.is_empty() {
        state.fence_floor.insert(resource.to_string(), rec.fence);
        state.claims.remove(resource);
    } else {
        state.claims.insert(resource.to_string(), rec);
    }
}

fn held_claims_index(
    state: &State,
    now: i64,
) -> std::collections::BTreeMap<String, Vec<String>> {
    let mut index: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();
    for rec in state.claims.values() {
        for h in &rec.holders {
            if h.lease_expires_at > now {
                index.entry(h.agent_id.clone()).or_default().push(rec.resource.clone());
            }
        }
    }
    index
}

fn claim_view(rec: &ClaimRecord) -> Value {
    let holders: Vec<String> = rec.holders.iter().map(|h| h.agent_id.clone()).collect();
    let claim_id = rec.holders.first().map(|h| h.claim_id.clone()).unwrap_or_default();
    let min_lease = rec
        .holders
        .iter()
        .map(|h| h.lease_expires_at)
        .min()
        .unwrap_or(0);
    let queue: Vec<Value> = rec
        .queue
        .iter()
        .enumerate()
        .map(|(i, t)| json!({ "agent_id": t.agent_id, "claim_id": t.claim_id, "position": i + 1 }))
        .collect();
    let mut view = json!({
        "resource": rec.resource,
        "mode": rec.mode,
        "holder": holders,
        "claim_id": claim_id,
        "fence": rec.fence,
        "lease_expires_at": iso_from_unix(min_lease),
        "queue": queue,
    });
    if let Some(note) = rec.holders.first().and_then(|h| h.note.clone()) {
        view["note"] = json!(note);
    }
    view
}

fn addressed_to(recipient: &str, agent_id: &str, topics: &[String]) -> bool {
    if recipient == agent_id || recipient == "*" {
        return true;
    }
    if let Some(name) = recipient.strip_prefix("topic:") {
        return topics.iter().any(|t| {
            let t_name = t.strip_prefix("topic:").unwrap_or(t);
            t_name == name
        });
    }
    false
}

/// Ordered message_ids addressed to `agent_id`, id strictly above `cursor`, unexpired.
fn pending_after(
    state: &State,
    agent_id: &str,
    cursor: &str,
    topics: &[String],
    now: i64,
) -> Vec<String> {
    let mut rows: Vec<&LogEntry> = state
        .log
        .iter()
        .filter(|row| row.message_id.as_str() > cursor)
        .filter(|row| !row.expires_unix.map(|e| e <= now).unwrap_or(false))
        .filter(|row| addressed_to(&row.recipient, agent_id, topics))
        .collect();
    rows.sort_by(|a, b| a.message_id.cmp(&b.message_id));
    rows.into_iter().map(|r| r.message_id.clone()).collect()
}

fn to_inbox_message(rec: &Message) -> Value {
    let mut m = json!({
        "message_id": rec.message_id,
        "from": rec.from,
        "to": rec.to,
        "body": rec.body,
        "posted_at": iso_from_unix(rec.posted_at),
    });
    if let Some(subject) = &rec.subject {
        m["subject"] = json!(subject);
    }
    if let Some(reply_to) = &rec.reply_to {
        m["reply_to"] = json!(reply_to);
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex;

    struct Fixture {
        mesh: Mesh,
        _tmp: std::path::PathBuf,
        clock: Arc<Mutex<i64>>,
    }

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn fixture() -> Fixture {
        let mut tmp = std::env::temp_dir();
        let uniq = format!(
            "hub-test-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        tmp.push(uniq);
        let clock = Arc::new(Mutex::new(1_000_000i64));
        let c2 = clock.clone();
        let now: Clock = Arc::new(move || *c2.lock().unwrap());
        let dir = tmp.join(".mesh");
        let mesh = Mesh::open_with_clock(&dir, now).unwrap();
        Fixture { mesh, _tmp: tmp, clock }
    }

    impl Fixture {
        fn advance(&self, s: i64) {
            *self.clock.lock().unwrap() += s;
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self._tmp);
        }
    }

    #[test]
    fn register_roster_liveness_state_machine() {
        let f = fixture();
        let r = f
            .mesh
            .register(&json!({"agent_id":"a","branch":"gitfs/a","prompt_ptr":"p","ttl_seconds":60}))
            .unwrap();
        assert_eq!(r["epoch"], 1, "first register epoch=1");
        let roster = f.mesh.roster(&json!({"agent_id":"a"})).unwrap();
        assert_eq!(roster["agents"][0]["liveness"], "alive");

        f.advance(75); // past ttl into grace
        let roster = f.mesh.roster(&json!({"agent_id":"a","include_stale":true})).unwrap();
        assert_eq!(roster["agents"][0]["liveness"], "stale");

        f.advance(30); // past grace -> dead
        let roster = f.mesh.roster(&json!({"agent_id":"a"})).unwrap();
        assert_eq!(roster["agents"].as_array().unwrap().len(), 0, "dead hidden");
        let roster = f.mesh.roster(&json!({"agent_id":"a","include_stale":true})).unwrap();
        assert_eq!(roster["agents"][0]["liveness"], "dead");

        let r2 = f
            .mesh
            .register(&json!({"agent_id":"a","branch":"gitfs/a","prompt_ptr":"p"}))
            .unwrap();
        assert_eq!(r2["epoch"], 2, "re-register after death bumps epoch");
    }

    #[test]
    fn exclusive_grant_deny_queue_promote_fence() {
        let f = fixture();
        let g = f.mesh.claim(&json!({"agent_id":"a","resource":"R"})).unwrap();
        assert_eq!(g["status"], "granted");
        assert_eq!(g["fence"], 1);
        let d = f.mesh.claim(&json!({"agent_id":"b","resource":"R"})).unwrap();
        assert_eq!(d["status"], "denied");
        assert_eq!(d["holder"], "a");
        let q = f
            .mesh
            .claim(&json!({"agent_id":"c","resource":"R","wait":"queue"}))
            .unwrap();
        assert_eq!(q["status"], "queued");
        assert_eq!(q["queue_position"], 1);
        let rel = f
            .mesh
            .release(&json!({"agent_id":"a","claim_id":g["claim_id"],"resource":"R"}))
            .unwrap();
        assert_eq!(rel["status"], "released");
        assert_eq!(rel["next_holder"], "c");
        let view = f.mesh.claims(&json!({"agent_id":"x"})).unwrap();
        assert_eq!(view["claims"][0]["holder"], json!(["c"]));
        assert_eq!(view["claims"][0]["fence"], 2);
    }

    #[test]
    fn idempotent_renewal_keeps_claim_id_and_fence() {
        let f = fixture();
        let g = f
            .mesh
            .claim(&json!({"agent_id":"a","resource":"R","lease_seconds":100}))
            .unwrap();
        f.advance(50);
        let g2 = f
            .mesh
            .claim(&json!({"agent_id":"a","resource":"R","lease_seconds":100}))
            .unwrap();
        assert_eq!(g2["claim_id"], g["claim_id"]);
        assert_eq!(g2["fence"], g["fence"]);
    }

    #[test]
    fn shared_co_holders_exclusive_blocked() {
        let f = fixture();
        assert_eq!(
            f.mesh.claim(&json!({"agent_id":"a","resource":"S","mode":"shared"})).unwrap()["status"],
            "granted"
        );
        assert_eq!(
            f.mesh.claim(&json!({"agent_id":"b","resource":"S","mode":"shared"})).unwrap()["status"],
            "granted"
        );
        assert_eq!(
            f.mesh.claim(&json!({"agent_id":"c","resource":"S","mode":"exclusive"})).unwrap()["status"],
            "denied"
        );
        let v = f.mesh.claims(&json!({"agent_id":"x"})).unwrap();
        let mut holders: Vec<String> = v["claims"][0]["holder"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap().to_string())
            .collect();
        holders.sort();
        assert_eq!(holders, vec!["a", "b"]);
    }

    #[test]
    fn lease_expiry_frees_lock() {
        let f = fixture();
        let g = f
            .mesh
            .claim(&json!({"agent_id":"a","resource":"R","lease_seconds":10}))
            .unwrap();
        f.advance(20);
        let g2 = f.mesh.claim(&json!({"agent_id":"b","resource":"R"})).unwrap();
        assert_eq!(g2["status"], "granted");
        assert!(g2["fence"].as_i64().unwrap() > g["fence"].as_i64().unwrap());
    }

    #[test]
    fn dead_agent_claim_self_heal() {
        let f = fixture();
        f.mesh
            .register(&json!({"agent_id":"a","branch":"gitfs/a","prompt_ptr":"p","ttl_seconds":60}))
            .unwrap();
        f.mesh
            .claim(&json!({"agent_id":"a","resource":"R","lease_seconds":10000}))
            .unwrap();
        f.advance(200); // a is dead (60 + 30 grace)
        assert_eq!(
            f.mesh.claim(&json!({"agent_id":"b","resource":"R"})).unwrap()["status"],
            "granted"
        );
    }

    #[test]
    fn not_holder_vs_unknown_on_release() {
        let f = fixture();
        let g = f.mesh.claim(&json!({"agent_id":"a","resource":"R"})).unwrap();
        assert_eq!(
            f.mesh.release(&json!({"agent_id":"b","claim_id":g["claim_id"],"resource":"R"})).unwrap()["status"],
            "not_holder"
        );
        assert_eq!(
            f.mesh.release(&json!({"agent_id":"a","claim_id":"ZZZ","resource":"R"})).unwrap()["status"],
            "unknown"
        );
        assert_eq!(
            f.mesh.release(&json!({"agent_id":"a","claim_id":"ZZZ","resource":"NOPE"})).unwrap()["status"],
            "unknown"
        );
    }

    #[test]
    fn post_inbox_read_cursor_advance() {
        let f = fixture();
        let p = f.mesh.post(&json!({"agent_id":"a","to":"b","body":"hi"})).unwrap();
        assert_eq!(p["fanout"], 1);
        assert_eq!(
            f.mesh.inbox(&json!({"agent_id":"c"})).unwrap()["messages"].as_array().unwrap().len(),
            0,
            "privacy: c does not see a->b"
        );
        let ib = f.mesh.inbox(&json!({"agent_id":"b"})).unwrap();
        assert_eq!(ib["messages"].as_array().unwrap().len(), 1);
        assert_eq!(ib["messages"][0]["body"], "hi");
        assert_eq!(ib["unread"], 0);
        f.mesh.read(&json!({"agent_id":"b","up_to":p["message_id"]})).unwrap();
        assert_eq!(
            f.mesh.inbox(&json!({"agent_id":"b"})).unwrap()["messages"].as_array().unwrap().len(),
            0,
            "cursor consumes exactly once"
        );
    }

    #[test]
    fn broadcast_and_topic_addressing() {
        let f = fixture();
        f.mesh.register(&json!({"agent_id":"a","branch":"x","prompt_ptr":"p"})).unwrap();
        f.mesh.register(&json!({"agent_id":"b","branch":"x","prompt_ptr":"p"})).unwrap();
        let bc = f.mesh.post(&json!({"agent_id":"a","to":"*","body":"all"})).unwrap();
        assert_eq!(bc["fanout"], 2);
        assert_eq!(
            f.mesh.inbox(&json!({"agent_id":"b"})).unwrap()["messages"].as_array().unwrap().len(),
            1
        );
        f.mesh.post(&json!({"agent_id":"a","to":"topic:build","body":"t"})).unwrap();
        let none = f.mesh.inbox(&json!({"agent_id":"b"})).unwrap();
        let topic_msgs: Vec<_> = none["messages"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|m| m["to"] == "topic:build")
            .collect();
        assert_eq!(topic_msgs.len(), 0, "no topic without subscription");
        let sub = f.mesh.inbox(&json!({"agent_id":"b","topics":["build"]})).unwrap();
        let topic_msgs: Vec<_> = sub["messages"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|m| m["to"] == "topic:build")
            .collect();
        assert_eq!(topic_msgs.len(), 1, "topic delivered to subscriber");
    }

    #[test]
    fn gc_reclaims_expired_messages() {
        let f = fixture();
        f.mesh.post(&json!({"agent_id":"a","to":"*","body":"early"})).unwrap();
        assert_eq!(
            f.mesh.inbox(&json!({"agent_id":"late"})).unwrap()["messages"].as_array().unwrap().len(),
            1
        );
        f.mesh.post(&json!({"agent_id":"a","to":"late","body":"ttl","ttl_seconds":10})).unwrap();
        assert_eq!(
            f.mesh.inbox(&json!({"agent_id":"late"})).unwrap()["messages"].as_array().unwrap().len(),
            2
        );
        f.advance(20);
        assert_eq!(
            f.mesh.inbox(&json!({"agent_id":"late"})).unwrap()["messages"].as_array().unwrap().len(),
            1
        );
        let reclaimed = f.mesh.gc().unwrap();
        assert_eq!(reclaimed, 1, "gc reclaims one expired message");
    }

    #[test]
    fn state_persists_across_restart() {
        let f = fixture();
        let g = f.mesh.claim(&json!({"agent_id":"a","resource":"R"})).unwrap();
        let dir = f._tmp.join(".mesh");
        let clock: Clock = Arc::new(|| 1_000_000);
        let m2 = Mesh::open_with_clock(&dir, clock).unwrap();
        let v = m2.claims(&json!({"agent_id":"x"})).unwrap();
        assert_eq!(v["claims"][0]["holder"], json!(["a"]));
        assert_eq!(v["claims"][0]["claim_id"], g["claim_id"]);
    }

    #[test]
    fn iso_format_no_millis() {
        assert_eq!(iso_from_unix(1_760_000_000), "2025-10-09T08:53:20Z");
        // epoch
        assert_eq!(iso_from_unix(0), "1970-01-01T00:00:00Z");
    }
}
