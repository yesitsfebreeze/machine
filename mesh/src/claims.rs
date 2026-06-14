//! Atomic claims / locks (SPEC §5, the core value of the daemon).
//!
//! Every grant, release, expiry, and queue promotion happens inside a single
//! LMDB write transaction (S-2). LMDB is single-writer with a process-shared
//! lock file, so when two OS processes race for the same exclusive resource,
//! exactly one write txn commits the grant first and the other observes the held
//! state — the cross-process CAS git cannot provide (C-3).
//!
//! Self-healing: before any claim mutation, and on `roster`/`claims` reads, the
//! daemon `sweep`s expired leases (C-12) and dead agents (C-13), freeing locks
//! and promoting waiters. This makes a crashed holder unable to hold a lock
//! forever without requiring a background thread.

use std::collections::HashMap;

use heed::RoTxn;
use ulid::Ulid;

use crate::clock::iso_from_unix;
use crate::error::{MeshError, Result};
use crate::records::{ClaimRecord, Holder, QueuedTicket, RosterRecord};
use crate::roster::liveness_at;
use crate::store::{decode, encode};
use crate::types::{
    ClaimMode, ClaimRequest, ClaimResponse, ClaimStatus, ClaimView, ClaimsRequest, ClaimsResponse,
    Liveness, QueueTicket, ReleaseRequest, ReleaseResponse, ReleaseStatus,
};
use crate::Daemon;

/// Default lease window for a granted hold if the caller omits `lease_seconds`.
pub const DEFAULT_LEASE_SECONDS: i64 = 120;

impl Daemon {
    /// `claim` — atomically acquire, renew, or queue for a resource (SPEC §5.1-§5.3).
    pub fn claim(&self, req: ClaimRequest) -> Result<ClaimResponse> {
        if req.agent_id.is_empty() || req.resource.is_empty() {
            return Err(MeshError::BadRequest(
                "agent_id and resource are required".into(),
            ));
        }
        let now = self.clock.now_unix();
        let lease = req.lease_seconds.unwrap_or(DEFAULT_LEASE_SECONDS).max(1);

        let store = &self.store;
        let mut wtxn = store.env().write_txn()?;
        let db = store.claims_db();

        // Reap expired/dead state inside this same txn so the grant decision sees
        // a freed lock atomically (C-12/C-13). A fresh record resumes from the
        // persisted fence floor so fence is monotonic per resource across the
        // resource's whole life — even after the record was deleted while idle
        // (C-4).
        let mut record = match db.get(&wtxn, &req.resource)? {
            Some(bytes) => decode::<ClaimRecord>(bytes)?,
            None => ClaimRecord {
                resource: req.resource.clone(),
                mode: req.mode,
                holders: Vec::new(),
                queue: Vec::new(),
                fence: self.fence_floor(&wtxn, &req.resource)?,
            },
        };
        self.reap_record(&wtxn, &mut record, now)?;

        // Idempotent renewal by the current holder (C-14/C-15).
        if let Some(pos) = record.holders.iter().position(|h| h.agent_id == req.agent_id) {
            let holder = &mut record.holders[pos];
            holder.lease_expires_at = now + lease;
            if req.note.is_some() {
                holder.note = req.note.clone();
            }
            let claim_id = holder.claim_id.clone();
            let lease_expires = holder.lease_expires_at;
            let fence = record.fence;
            db.put(&mut wtxn, &req.resource, &encode(&record)?)?;
            wtxn.commit()?;
            return Ok(ClaimResponse {
                status: ClaimStatus::Granted,
                resource: req.resource,
                claim_id,
                holder: Some(req.agent_id),
                lease_expires_at: Some(iso_from_unix(lease_expires)?),
                queue_position: None,
                fence,
            });
        }

        let can_grant = grantable(&record, req.mode);
        if can_grant {
            let claim_id = Ulid::new().to_string();
            record.mode = req.mode;
            record.fence += 1;
            record.holders.push(Holder {
                agent_id: req.agent_id.clone(),
                claim_id: claim_id.clone(),
                lease_expires_at: now + lease,
                note: req.note.clone(),
            });
            let fence = record.fence;
            db.put(&mut wtxn, &req.resource, &encode(&record)?)?;
            self.journal_claim_event(&req.resource, "grant", &req.agent_id, fence, now)?;
            wtxn.commit()?;
            return Ok(ClaimResponse {
                status: ClaimStatus::Granted,
                resource: req.resource,
                claim_id,
                holder: Some(req.agent_id),
                lease_expires_at: Some(iso_from_unix(now + lease)?),
                queue_position: None,
                fence,
            });
        }

        // Held by someone else and not grantable: queue or deny.
        let current_holder = record.holders.first().map(|h| h.agent_id.clone());
        match req.wait {
            crate::types::WaitPolicy::Queue => {
                let claim_id = Ulid::new().to_string();
                record.queue.push(QueuedTicket {
                    agent_id: req.agent_id.clone(),
                    claim_id: claim_id.clone(),
                    mode: req.mode,
                    lease_seconds: lease,
                    note: req.note.clone(),
                });
                let position = record.queue.len() as i64;
                let fence = record.fence;
                db.put(&mut wtxn, &req.resource, &encode(&record)?)?;
                wtxn.commit()?;
                Ok(ClaimResponse {
                    status: ClaimStatus::Queued,
                    resource: req.resource,
                    claim_id,
                    holder: current_holder,
                    lease_expires_at: None,
                    queue_position: Some(position),
                    fence,
                })
            }
            crate::types::WaitPolicy::NoWait => {
                let fence = record.fence;
                // Persist any reaping done above even though we grant nothing new.
                db.put(&mut wtxn, &req.resource, &encode(&record)?)?;
                wtxn.commit()?;
                Ok(ClaimResponse {
                    status: ClaimStatus::Denied,
                    resource: req.resource,
                    claim_id: String::new(),
                    holder: current_holder,
                    lease_expires_at: None,
                    queue_position: None,
                    fence,
                })
            }
        }
    }

    /// `release` — relinquish a hold or cancel a queued ticket (SPEC §5.4).
    pub fn release(&self, req: ReleaseRequest) -> Result<ReleaseResponse> {
        let now = self.clock.now_unix();
        let store = &self.store;
        let mut wtxn = store.env().write_txn()?;
        let db = store.claims_db();

        let mut record: ClaimRecord = match db.get(&wtxn, &req.resource)? {
            Some(bytes) => decode(bytes)?,
            None => {
                wtxn.commit()?;
                return Ok(ReleaseResponse {
                    status: ReleaseStatus::Unknown,
                    next_holder: None,
                    fence: None,
                });
            }
        };

        let holder_idx = record
            .holders
            .iter()
            .position(|h| h.agent_id == req.agent_id && h.claim_id == req.claim_id);
        let queue_idx = record
            .queue
            .iter()
            .position(|t| t.agent_id == req.agent_id && t.claim_id == req.claim_id);

        if holder_idx.is_none() && queue_idx.is_none() {
            // Either unknown claim_id, or a non-holder attempt (T-2/C-11).
            let known = record.holders.iter().any(|h| h.claim_id == req.claim_id)
                || record.queue.iter().any(|t| t.claim_id == req.claim_id);
            wtxn.commit()?;
            return Ok(ReleaseResponse {
                status: if known {
                    ReleaseStatus::NotHolder
                } else {
                    ReleaseStatus::Unknown
                },
                next_holder: None,
                fence: None,
            });
        }

        if let Some(qi) = queue_idx {
            // Cancel a queued ticket: no promotion, hold unchanged.
            record.queue.remove(qi);
            db.put(&mut wtxn, &req.resource, &encode(&record)?)?;
            wtxn.commit()?;
            return Ok(ReleaseResponse {
                status: ReleaseStatus::Released,
                next_holder: None,
                fence: Some(record.fence),
            });
        }

        // Holder release: drop the hold, then promote the queue (C-8/C-10).
        let hi = holder_idx.expect("holder_idx checked above");
        record.holders.remove(hi);
        self.journal_claim_event(&req.resource, "release", &req.agent_id, record.fence, now)?;
        let promoted = promote_queue(&mut record, now);
        if let Some(ref new_holder) = promoted {
            self.journal_claim_event(&req.resource, "promote", new_holder, record.fence, now)?;
        }

        let next_holder = promoted;
        let fence = record.fence;
        self.persist_or_delete(&mut wtxn, &req.resource, &record)?;
        wtxn.commit()?;
        Ok(ReleaseResponse {
            status: ReleaseStatus::Released,
            next_holder,
            fence: Some(fence),
        })
    }

    /// `claims` — inspect live locks and queues (SPEC §4.2).
    pub fn claims(&self, req: ClaimsRequest) -> Result<ClaimsResponse> {
        let now = self.clock.now_unix();
        self.sweep(now)?;

        let store = &self.store;
        let rtxn = store.env().read_txn()?;
        let db = store.claims_db();

        let mut out = Vec::new();
        for item in db.iter(&rtxn)? {
            let (_key, bytes) = item?;
            let rec: ClaimRecord = decode(bytes)?;
            if rec.is_free() {
                continue;
            }
            if let Some(ref filter) = req.resource {
                if &rec.resource != filter {
                    continue;
                }
            }
            out.push(claim_view(&rec)?);
        }
        out.sort_by(|a, b| a.resource.cmp(&b.resource));
        Ok(ClaimsResponse { claims: out })
    }

    // --- self-healing helpers ---------------------------------------------

    /// Reap expired leases (C-12) and dead-agent holds (C-13) across every
    /// resource, promoting waiters. Runs in its own write txn before reporting
    /// reads, and is also invoked inline within `claim`'s txn via `reap_record`.
    pub fn sweep(&self, now: i64) -> Result<()> {
        let store = &self.store;
        let mut wtxn = store.env().write_txn()?;
        let cdb = store.claims_db();

        // Snapshot keys first to avoid mutating while iterating.
        let mut keys: Vec<String> = Vec::new();
        for item in cdb.iter(&wtxn)? {
            let (key, _bytes) = item?;
            keys.push(key.to_string());
        }

        for key in keys {
            let mut rec: ClaimRecord = match cdb.get(&wtxn, &key)? {
                Some(b) => decode(b)?,
                None => continue,
            };
            let changed = self.reap_record(&wtxn, &mut rec, now)?;
            if changed {
                self.persist_or_delete(&mut wtxn, &key, &rec)?;
            }
        }
        wtxn.commit()?;
        Ok(())
    }

    /// Reap one record in-place against `now`: drop holders whose lease expired
    /// or whose agent is dead, drop queued tickets owned by dead agents, then
    /// promote waiters. Returns whether the record changed. Reads roster within
    /// the supplied txn so the decision is transactionally consistent.
    fn reap_record(
        &self,
        txn: &heed::RwTxn,
        rec: &mut ClaimRecord,
        now: i64,
    ) -> Result<bool> {
        let before_holders = rec.holders.len();
        let before_queue = rec.queue.len();

        // Drop expired or dead holders.
        let rdb = self.store.roster_db();
        rec.holders.retain(|h| {
            if h.lease_expires_at <= now {
                return false; // lease expired (C-12)
            }
            !agent_is_dead(&rdb, txn, &h.agent_id, now)
        });

        // Drop queued tickets owned by dead agents (C-13).
        rec.queue
            .retain(|t| !agent_is_dead(&rdb, txn, &t.agent_id, now));

        let mut changed = rec.holders.len() != before_holders || rec.queue.len() != before_queue;

        // If the resource is now free but has waiters, promote.
        if rec.holders.is_empty() && !rec.queue.is_empty() && promote_queue(rec, now).is_some() {
            changed = true;
        }
        Ok(changed)
    }

    /// Index of `agent_id` -> sorted resource ids it currently holds, built in a
    /// single pass over the claims DB (for roster `held_claims`). Each agent's
    /// list is sorted so the roster output is stable.
    pub fn held_claims_index(
        &self,
        rtxn: &RoTxn,
        now: i64,
    ) -> Result<HashMap<String, Vec<String>>> {
        let cdb = self.store.claims_db();
        let mut index: HashMap<String, Vec<String>> = HashMap::new();
        for item in cdb.iter(rtxn)? {
            let (_key, bytes) = item?;
            let rec: ClaimRecord = decode(bytes)?;
            for holder in &rec.holders {
                if holder.lease_expires_at > now {
                    index
                        .entry(holder.agent_id.clone())
                        .or_default()
                        .push(rec.resource.clone());
                }
            }
        }
        for resources in index.values_mut() {
            resources.sort();
        }
        Ok(index)
    }

    /// Persist a claim record, or delete the key when the resource is fully idle
    /// (no holders, no queue) — but first stamp the resource's fence floor into
    /// the meta DB so a future re-acquire resumes monotonically (C-4).
    fn persist_or_delete(
        &self,
        wtxn: &mut heed::RwTxn,
        key: &str,
        rec: &ClaimRecord,
    ) -> Result<()> {
        let db = self.store.claims_db();
        if rec.holders.is_empty() && rec.queue.is_empty() {
            self.set_fence_floor(wtxn, key, rec.fence)?;
            db.delete(wtxn, key)?;
        } else {
            db.put(wtxn, key, &encode(rec)?)?;
        }
        Ok(())
    }

    /// Read the persisted fence floor for a resource (0 if none). Used to seed a
    /// freshly created record so fence never regresses.
    fn fence_floor(&self, wtxn: &heed::RwTxn, resource: &str) -> Result<i64> {
        let key = fence_floor_key(resource);
        match self.store.meta_db().get(wtxn, &key)? {
            Some(bytes) => Ok(decode(bytes)?),
            None => Ok(0),
        }
    }

    fn set_fence_floor(&self, wtxn: &mut heed::RwTxn, resource: &str, fence: i64) -> Result<()> {
        let key = fence_floor_key(resource);
        self.store
            .meta_db()
            .put(wtxn, &key, &encode(&fence)?)?;
        Ok(())
    }
}

/// Meta-DB key under which a resource's fence floor is stored.
fn fence_floor_key(resource: &str) -> String {
    format!("fence_floor:{resource}")
}

// --- free functions -------------------------------------------------------

/// Can a new claim of `mode` be granted given the current record state?
/// - Free resource: always grantable.
/// - Held shared + requesting shared: co-holder allowed (C-2).
/// - Otherwise (any exclusive involvement): not grantable (C-1/C-3).
fn grantable(record: &ClaimRecord, mode: ClaimMode) -> bool {
    if record.holders.is_empty() {
        return true;
    }
    matches!(
        (record.mode, mode),
        (ClaimMode::Shared, ClaimMode::Shared)
    )
}

/// Promote the oldest waiting ticket to holder, bumping the fence (C-8).
/// For a shared promotion, also pulls any contiguous shared waiters behind it.
/// Returns the agent_id of the (first) promoted holder, if any.
fn promote_queue(record: &mut ClaimRecord, now: i64) -> Option<String> {
    if !record.holders.is_empty() || record.queue.is_empty() {
        return None;
    }
    let ticket = record.queue.remove(0);
    record.mode = ticket.mode;
    record.fence += 1;
    let first = ticket.agent_id.clone();
    record.holders.push(holder_from_ticket(ticket, now));

    // For shared, absorb leading shared waiters so they share the grant.
    if record.mode == ClaimMode::Shared {
        while record.queue.first().map(|t| t.mode) == Some(ClaimMode::Shared) {
            let t = record.queue.remove(0);
            record.holders.push(holder_from_ticket(t, now));
        }
    }
    Some(first)
}

/// Build a [`Holder`] from a promoted queue ticket, anchoring its lease at `now`.
fn holder_from_ticket(ticket: QueuedTicket, now: i64) -> Holder {
    Holder {
        agent_id: ticket.agent_id,
        claim_id: ticket.claim_id,
        lease_expires_at: now + ticket.lease_seconds,
        note: ticket.note,
    }
}

/// Is `agent_id` past its dead threshold per the roster (C-13)?
/// An agent with no roster entry is treated as not-dead (it may be claiming
/// before its first heartbeat); only an entry that has crossed the dead window
/// triggers cleanup.
fn agent_is_dead(
    rdb: &heed::Database<heed::types::Str, heed::types::Bytes>,
    txn: &heed::RwTxn,
    agent_id: &str,
    now: i64,
) -> bool {
    match rdb.get(txn, agent_id) {
        Ok(Some(bytes)) => match decode::<RosterRecord>(bytes) {
            Ok(rec) => liveness_at(rec.expires_at, now) == Liveness::Dead,
            Err(_) => false,
        },
        _ => false,
    }
}

/// Build the wire `ClaimView` from a record.
fn claim_view(rec: &ClaimRecord) -> Result<ClaimView> {
    let holders: Vec<String> = rec.holders.iter().map(|h| h.agent_id.clone()).collect();
    let claim_id = rec
        .holders
        .first()
        .map(|h| h.claim_id.clone())
        .unwrap_or_default();
    let lease = rec
        .holders
        .iter()
        .map(|h| h.lease_expires_at)
        .min()
        .unwrap_or(0);
    let note = rec.holders.first().and_then(|h| h.note.clone());
    let queue = rec
        .queue
        .iter()
        .enumerate()
        .map(|(i, t)| QueueTicket {
            agent_id: t.agent_id.clone(),
            claim_id: t.claim_id.clone(),
            position: (i + 1) as i64,
        })
        .collect();
    Ok(ClaimView {
        resource: rec.resource.clone(),
        mode: rec.mode,
        holder: holders,
        claim_id,
        fence: rec.fence,
        lease_expires_at: iso_from_unix(lease)?,
        queue,
        note,
    })
}
