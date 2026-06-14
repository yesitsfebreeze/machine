//! Roster and liveness (SPEC §6). `register` is an idempotent upsert keyed by
//! `agent_id` that also serves as the heartbeat. `roster` lists known agents with
//! derived liveness and their held claims.
//!
//! Liveness is derived purely from heartbeat freshness (R-6): never from git
//! commit timestamps. A quiet agent that keeps heartbeating is alive.

use crate::clock::iso_from_unix;
use crate::error::Result;
use crate::records::RosterRecord;
use crate::store::{decode, encode};
use crate::types::{
    Liveness, RegisterRequest, RegisterResponse, RosterEntry, RosterRequest, RosterResponse,
};
use crate::Daemon;

/// Default liveness window if a caller omits `ttl_seconds`.
pub const DEFAULT_TTL_SECONDS: i64 = 60;
/// Grace window after `expires_at` during which an agent reads as `stale`
/// before becoming `dead` (SPEC R-3).
pub const STALE_GRACE_SECONDS: i64 = 30;

/// Derive liveness from `now` against an entry's `expires_at` (SPEC R-3).
pub fn liveness_at(expires_at: i64, now: i64) -> Liveness {
    if now <= expires_at {
        Liveness::Alive
    } else if now <= expires_at + STALE_GRACE_SECONDS {
        Liveness::Stale
    } else {
        Liveness::Dead
    }
}

impl Daemon {
    /// `register` — announce presence / heartbeat (SPEC §4.1, R-1/R-2/R-5).
    pub fn register(&self, req: RegisterRequest) -> Result<RegisterResponse> {
        if req.agent_id.is_empty() {
            return Err(crate::error::MeshError::BadRequest(
                "agent_id is required".into(),
            ));
        }
        let now = self.clock.now_unix();
        let ttl = req.ttl_seconds.unwrap_or(DEFAULT_TTL_SECONDS).max(1);

        let store = &self.store;
        let mut wtxn = store.env().write_txn()?;
        let db = store.roster_db();

        let existing: Option<RosterRecord> = match db.get(&wtxn, &req.agent_id)? {
            Some(bytes) => Some(decode(bytes)?),
            None => None,
        };

        let record = match existing {
            Some(prev) => {
                // Re-registration after death bumps epoch so peers see a restart (R-5).
                let was_dead = liveness_at(prev.expires_at, now) == Liveness::Dead;
                RosterRecord {
                    agent_id: req.agent_id.clone(),
                    branch: req.branch,
                    prompt_ptr: req.prompt_ptr,
                    role: req.role,
                    registered_at: prev.registered_at,
                    last_seen: now,
                    expires_at: now + ttl,
                    epoch: if was_dead { prev.epoch + 1 } else { prev.epoch },
                }
            }
            None => RosterRecord {
                agent_id: req.agent_id.clone(),
                branch: req.branch,
                prompt_ptr: req.prompt_ptr,
                role: req.role,
                registered_at: now,
                last_seen: now,
                expires_at: now + ttl,
                epoch: 1,
            },
        };

        db.put(&mut wtxn, &req.agent_id, &encode(&record)?)?;
        wtxn.commit()?;

        Ok(RegisterResponse {
            agent_id: record.agent_id,
            registered_at: iso_from_unix(record.registered_at)?,
            expires_at: iso_from_unix(record.expires_at)?,
            epoch: record.epoch,
        })
    }

    /// `roster` — list known agents with derived liveness (SPEC §4.1).
    /// Self-heals dead agents' claims first (C-13) so `held_claims` is accurate.
    pub fn roster(&self, req: RosterRequest) -> Result<RosterResponse> {
        let now = self.clock.now_unix();
        // Reap expired/dead state before reporting so the view is consistent.
        self.sweep(now)?;

        let store = &self.store;
        let rtxn = store.env().read_txn()?;
        let db = store.roster_db();

        // Build the agent -> held-resources index in a single pass over the
        // claims DB, instead of re-scanning it once per agent.
        let mut held_by = self.held_claims_index(&rtxn, now)?;

        let mut agents = Vec::new();
        for item in db.iter(&rtxn)? {
            let (_key, bytes) = item?;
            let rec: RosterRecord = decode(bytes)?;
            let liveness = liveness_at(rec.expires_at, now);
            if liveness == Liveness::Dead && !req.include_stale {
                continue;
            }
            let held = held_by.remove(&rec.agent_id).unwrap_or_default();
            agents.push(RosterEntry {
                agent_id: rec.agent_id,
                branch: rec.branch,
                prompt_ptr: rec.prompt_ptr,
                role: rec.role,
                liveness,
                last_seen: iso_from_unix(rec.last_seen)?,
                expires_at: iso_from_unix(rec.expires_at)?,
                held_claims: held,
            });
        }
        agents.sort_by(|a, b| a.agent_id.cmp(&b.agent_id));
        Ok(RosterResponse { agents })
    }
}
