//! Durable messaging (SPEC §9). A message is stored once (LMDB body + SQLite log
//! row) regardless of fanout; each recipient reads through its own cursor (M-8).
//!
//! - Addressing (M-1): a specific `agent_id`, `*` (broadcast), or `topic:<name>`.
//! - Durability (M-4): a message survives sender death and is deliverable to a
//!   recipient that did not exist at post time. The store does not depend on any
//!   recipient polling (D-3).
//! - Cursor (M-5): a per-`agent_id` cursor in the SQLite journal makes `inbox`
//!   return each message at most once per recipient and survives restart.
//! - Ordering: `message_id` is a ULID — sortable and time-ordered — giving a total
//!   order per recipient cursor.

use ulid::Ulid;

use crate::clock::iso_from_unix;
use crate::error::{MeshError, Result};
use crate::records::MessageRecord;
use crate::store::{decode, encode};
use crate::types::{
    InboxMessage, InboxRequest, InboxResponse, PostRequest, PostResponse, ReadRequest, ReadResponse,
};
use crate::Daemon;

/// The empty cursor: lexicographically below every ULID, so a brand-new agent
/// sees all still-pending messages it is addressed by (supports late joiners, M-4).
const ZERO_CURSOR: &str = "00000000000000000000000000";

impl Daemon {
    /// `post` — store a durable message and log it (SPEC §4.3, M-3/M-8).
    pub fn post(&self, req: PostRequest) -> Result<PostResponse> {
        if req.agent_id.is_empty() || req.to.is_empty() || req.body.is_empty() {
            return Err(MeshError::BadRequest(
                "agent_id, to, and body are required".into(),
            ));
        }
        let now = self.clock.now_unix();
        let message_id = Ulid::new().to_string();
        let expires_at = req.ttl_seconds.map(|ttl| now + ttl.max(1));

        let record = MessageRecord {
            message_id: message_id.clone(),
            from: req.agent_id.clone(),
            to: req.to.clone(),
            subject: req.subject.clone(),
            body: req.body.clone(),
            reply_to: req.reply_to.clone(),
            posted_at: now,
            expires_at,
        };

        // Body in LMDB (single copy), keyed by message_id.
        {
            let store = &self.store;
            let mut wtxn = store.env().write_txn()?;
            store
                .messages_db()
                .put(&mut wtxn, &message_id, &encode(&record)?)?;
            wtxn.commit()?;
        }

        // Ordered log row in SQLite for cursor enumeration.
        {
            let conn = self.store.journal()?;
            conn.execute(
                "INSERT INTO message_log (message_id, sender, recipient, posted_at, expires_unix)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    message_id,
                    req.agent_id,
                    req.to,
                    iso_from_unix(now)?,
                    expires_at,
                ],
            )?;
        }

        let fanout = self.fanout_for(&req.to, now)?;
        Ok(PostResponse {
            message_id,
            posted_at: iso_from_unix(now)?,
            fanout,
        })
    }

    /// `inbox` — peek pending messages without advancing the cursor (SPEC §4.3, D-2).
    pub fn inbox(&self, req: InboxRequest) -> Result<InboxResponse> {
        if req.agent_id.is_empty() {
            return Err(MeshError::BadRequest("agent_id is required".into()));
        }
        let now = self.clock.now_unix();
        let cursor = match &req.since {
            Some(s) => s.clone(),
            None => self.cursor_for(&req.agent_id)?,
        };
        let limit = req.limit.unwrap_or(100).clamp(1, 1000);

        let pending = self.pending_after(&req.agent_id, &cursor, &req.topics, now)?;
        let total = pending.len() as i64;

        let mut messages = Vec::new();
        for id in pending.iter().take(limit as usize) {
            if let Some(msg) = self.load_message(id)? {
                messages.push(to_inbox_message(&msg)?);
            }
        }
        let returned = messages.len() as i64;
        let live_cursor = self.cursor_for(&req.agent_id)?;
        Ok(InboxResponse {
            messages,
            cursor: live_cursor,
            unread: (total - returned).max(0),
        })
    }

    /// `read` — advance the caller's cursor (SPEC §4.3, M-5). Only the caller's
    /// own cursor is touched (T-3 privacy is structural: cursor is keyed by the
    /// request's `agent_id`).
    pub fn read(&self, req: ReadRequest) -> Result<ReadResponse> {
        if req.agent_id.is_empty() || req.up_to.is_empty() {
            return Err(MeshError::BadRequest(
                "agent_id and up_to are required".into(),
            ));
        }
        let current = self.cursor_for(&req.agent_id)?;
        // Cursor only ever moves forward.
        let new_cursor = if req.up_to > current {
            req.up_to.clone()
        } else {
            current
        };
        {
            let conn = self.store.journal()?;
            conn.execute(
                "INSERT INTO read_cursors (agent_id, cursor) VALUES (?1, ?2)
                 ON CONFLICT(agent_id) DO UPDATE SET cursor = excluded.cursor",
                rusqlite::params![req.agent_id, new_cursor],
            )?;
        }
        let now = self.clock.now_unix();
        let remaining = self
            .pending_after(&req.agent_id, &new_cursor, &[], now)?
            .len() as i64;
        Ok(ReadResponse {
            cursor: new_cursor,
            remaining,
        })
    }

    // --- helpers -----------------------------------------------------------

    /// The caller's stored cursor, or the zero cursor if none yet.
    fn cursor_for(&self, agent_id: &str) -> Result<String> {
        let conn = self.store.journal()?;
        let cursor: Option<String> = conn
            .query_row(
                "SELECT cursor FROM read_cursors WHERE agent_id = ?1",
                rusqlite::params![agent_id],
                |row| row.get(0),
            )
            .ok();
        Ok(cursor.unwrap_or_else(|| ZERO_CURSOR.to_string()))
    }

    /// Ordered message_ids addressed to `agent_id` (point-to-point, `*`, and the
    /// caller's requested `topics`) with id strictly greater than `cursor` and not
    /// expired. Drives both `inbox` and `read.remaining`.
    fn pending_after(
        &self,
        agent_id: &str,
        cursor: &str,
        topics: &[String],
        now: i64,
    ) -> Result<Vec<String>> {
        let conn = self.store.journal()?;
        let mut stmt = conn.prepare(
            "SELECT message_id, recipient, expires_unix FROM message_log
             WHERE message_id > ?1 ORDER BY message_id ASC",
        )?;
        let rows = stmt.query_map(rusqlite::params![cursor], |row| {
            let id: String = row.get(0)?;
            let recipient: String = row.get(1)?;
            let expires: Option<i64> = row.get(2)?;
            Ok((id, recipient, expires))
        })?;

        let mut out = Vec::new();
        for row in rows {
            let (id, recipient, expires) = row?;
            if let Some(exp) = expires {
                if exp <= now {
                    continue; // TTL-expired (M-6)
                }
            }
            if addressed_to(&recipient, agent_id, topics) {
                out.push(id);
            }
        }
        Ok(out)
    }

    /// Load a stored message body from LMDB by id (None if GC'd).
    fn load_message(&self, message_id: &str) -> Result<Option<MessageRecord>> {
        let rtxn = self.store.env().read_txn()?;
        match self.store.messages_db().get(&rtxn, message_id)? {
            Some(bytes) => Ok(Some(decode(bytes)?)),
            None => Ok(None),
        }
    }

    /// Count of distinct recipients a `to` address resolves to right now (M-1).
    /// `*` counts every known agent; `topic:` is unbounded by registration so it
    /// reports 0 named recipients (topic delivery is reader-driven, M-2); a direct
    /// address counts as 1.
    fn fanout_for(&self, to: &str, _now: i64) -> Result<i64> {
        if to == "*" {
            let rtxn = self.store.env().read_txn()?;
            let mut n = 0i64;
            for item in self.store.roster_db().iter(&rtxn)? {
                item?;
                n += 1;
            }
            Ok(n)
        } else if to.starts_with("topic:") {
            Ok(0)
        } else {
            Ok(1)
        }
    }
}

/// Does a logged `recipient` address reach `agent_id` given its `topics`?
/// - exact `agent_id` match (point-to-point, private)
/// - `*` broadcast reaches everyone
/// - `topic:<name>` reaches a reader that listed `<name>` in `topics`
fn addressed_to(recipient: &str, agent_id: &str, topics: &[String]) -> bool {
    if recipient == agent_id {
        return true;
    }
    if recipient == "*" {
        return true;
    }
    if let Some(name) = recipient.strip_prefix("topic:") {
        return topics.iter().any(|t| {
            let t = t.strip_prefix("topic:").unwrap_or(t);
            t == name
        });
    }
    false
}

fn to_inbox_message(rec: &MessageRecord) -> Result<InboxMessage> {
    Ok(InboxMessage {
        message_id: rec.message_id.clone(),
        from: rec.from.clone(),
        to: rec.to.clone(),
        subject: rec.subject.clone(),
        body: rec.body.clone(),
        posted_at: iso_from_unix(rec.posted_at)?,
        reply_to: rec.reply_to.clone(),
    })
}
