//! mesh — fleet inter-agent communication daemon (SPEC-COMM-001).
//!
//! kern's coordination sibling: kern owns *memory* (why decisions were made),
//! mesh owns *live coordination* (who is here, who holds what, who said what to
//! whom). Three categories of dynamic state — roster, claims, messages — over an
//! LMDB primary + SQLite-WAL journal, exposed as eight MCP verbs.
//!
//! The public surface is the [`Daemon`] type plus the wire `types`. The MCP
//! server ([`mcp`]) dispatches the eight verbs to `Daemon` methods.

pub mod claims;
pub mod clock;
pub mod error;
pub mod mcp;
pub mod messages;
pub mod records;
pub mod roster;
pub mod store;
pub mod types;

use std::path::Path;

use crate::clock::Clock;
use crate::error::Result;
use crate::store::Store;

/// The coordination daemon. Holds the storage handle and a clock. All eight
/// verbs are methods (implemented across the `roster`, `claims`, and `messages`
/// modules). Stateless beyond the store, so a fresh `Daemon` over the same data
/// dir resumes exactly where a previous one left off.
pub struct Daemon {
    pub(crate) store: Store,
    pub(crate) clock: Clock,
}

impl Daemon {
    /// Open (or create) the per-cwd data dir under `cwd` and attach the system clock.
    pub fn open_in(cwd: &Path) -> Result<Daemon> {
        Ok(Daemon {
            store: Store::open_in(cwd)?,
            clock: Clock::system(),
        })
    }

    /// Open directly under `dir` with an injectable `clock` (tests).
    pub fn open_with_clock(dir: &Path, clock: Clock) -> Result<Daemon> {
        Ok(Daemon {
            store: Store::open_dir(dir)?,
            clock,
        })
    }

    /// Test/maintenance accessor for advancing a fixed clock.
    pub fn clock(&self) -> &Clock {
        &self.clock
    }

    /// Append a claim-transition event to the journal (audit log, SPEC S-3).
    pub(crate) fn journal_claim_event(
        &self,
        resource: &str,
        event: &str,
        agent_id: &str,
        fence: i64,
        now: i64,
    ) -> Result<()> {
        let conn = self.store.journal()?;
        conn.execute(
            "INSERT INTO claim_events (resource, event, agent_id, fence, at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                resource,
                event,
                agent_id,
                fence,
                crate::clock::iso_from_unix(now)?
            ],
        )?;
        Ok(())
    }

    /// GC: drop TTL-expired messages from both stores and sweep dead claims
    /// (SPEC S-5). Returns the number of messages reclaimed. Live claims and
    /// unread messages are preserved.
    pub fn gc(&self) -> Result<usize> {
        let now = self.clock.now_unix();
        self.sweep(now)?;

        let conn = self.store.journal()?;
        // Collect expired ids so their LMDB bodies (keyed by message_id) can be
        // removed; the log rows themselves are cleared with one bulk DELETE.
        let expired: Vec<String> = {
            let mut stmt = conn.prepare(
                "SELECT message_id FROM message_log
                 WHERE expires_unix IS NOT NULL AND expires_unix <= ?1",
            )?;
            let ids = stmt
                .query_map(rusqlite::params![now], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            ids
        };

        // Remove bodies from LMDB.
        {
            let mut wtxn = self.store.env().write_txn()?;
            let mdb = self.store.messages_db();
            for id in &expired {
                mdb.delete(&mut wtxn, id)?;
            }
            wtxn.commit()?;
        }
        // Remove the corresponding log rows in a single statement.
        conn.execute(
            "DELETE FROM message_log WHERE expires_unix IS NOT NULL AND expires_unix <= ?1",
            rusqlite::params![now],
        )?;
        Ok(expired.len())
    }
}
