//! Storage layer: LMDB primary + SQLite-WAL journal (SPEC §3.2, S-1..S-5).
//!
//! - **LMDB** (`data.mdb`/`lock.mdb`) is the primary store. It holds the roster,
//!   live claims, and message bodies. A claim acquire/release is one LMDB write
//!   transaction, which is the cross-process atomic CAS the claim primitive needs
//!   (S-2). Records are JSON-encoded blobs keyed by id.
//! - **SQLite-WAL** (`journal/history.db`) is the append-only, time-ordered log:
//!   the global message log (for `inbox` replay), the claim-transition log (audit),
//!   and the per-agent read cursors (which survive restart). Cursors live here, not
//!   in LMDB, because they are naturally a small relational table with range reads.
//!
//! The data directory is per-cwd and gitignored (`.mesh/`, mirroring `.kern/`).

use std::path::{Path, PathBuf};

use heed::types::{Bytes, Str};
use heed::{Database, Env, EnvOpenOptions};
use rusqlite::Connection;
use serde::{de::DeserializeOwned, Serialize};

use crate::error::Result;

/// Per-cwd data directory name (gitignored), sibling to kern's `.kern/`.
pub const DATA_DIR: &str = ".mesh";

/// LMDB named databases (key spaces) within the single env.
const DB_ROSTER: &str = "roster"; // agent_id -> RosterRecord
const DB_CLAIMS: &str = "claims"; // resource -> ClaimRecord
const DB_MESSAGES: &str = "messages"; // message_id -> MessageRecord
const DB_META: &str = "meta"; // misc counters (fence floor, epoch seeds)

const MAX_DBS: u32 = 8;
/// 256 MiB map; ample for ephemeral coordination state, grows via remap on resize.
const MAP_SIZE: usize = 256 * 1024 * 1024;

/// Combined handle to both stores. Cheap to clone (Env is an Arc internally; the
/// SQLite connection is reopened per handle clone via the stored path).
pub struct Store {
    env: Env,
    roster: Database<Str, Bytes>,
    claims: Database<Str, Bytes>,
    messages: Database<Str, Bytes>,
    meta: Database<Str, Bytes>,
    journal_path: PathBuf,
}

impl Store {
    /// Resolve the data directory under `cwd` (creating it) and open both stores.
    pub fn open_in(cwd: &Path) -> Result<Store> {
        let dir = cwd.join(DATA_DIR);
        std::fs::create_dir_all(&dir)?;
        std::fs::create_dir_all(dir.join("journal"))?;
        Store::open_dir(&dir)
    }

    /// Open both stores directly under `dir` (used by tests with a tempdir).
    pub fn open_dir(dir: &Path) -> Result<Store> {
        std::fs::create_dir_all(dir)?;
        let journal_dir = dir.join("journal");
        std::fs::create_dir_all(&journal_dir)?;

        // SAFETY: heed/LMDB requires the caller to guarantee no other process has
        // the env open with an incompatible config. The per-cwd daemon model means
        // a single env per data dir; LMDB's own lock file arbitrates writers.
        let env = unsafe {
            EnvOpenOptions::new()
                .map_size(MAP_SIZE)
                .max_dbs(MAX_DBS)
                .open(dir)?
        };

        let mut wtxn = env.write_txn()?;
        let roster = env.create_database(&mut wtxn, Some(DB_ROSTER))?;
        let claims = env.create_database(&mut wtxn, Some(DB_CLAIMS))?;
        let messages = env.create_database(&mut wtxn, Some(DB_MESSAGES))?;
        let meta = env.create_database(&mut wtxn, Some(DB_META))?;
        wtxn.commit()?;

        let journal_path = journal_dir.join("history.db");
        let store = Store {
            env,
            roster,
            claims,
            messages,
            meta,
            journal_path,
        };
        store.init_journal()?;
        Ok(store)
    }

    // --- LMDB env access ---------------------------------------------------

    pub fn env(&self) -> &Env {
        &self.env
    }

    pub fn roster_db(&self) -> Database<Str, Bytes> {
        self.roster
    }

    pub fn claims_db(&self) -> Database<Str, Bytes> {
        self.claims
    }

    pub fn messages_db(&self) -> Database<Str, Bytes> {
        self.messages
    }

    pub fn meta_db(&self) -> Database<Str, Bytes> {
        self.meta
    }

    // --- SQLite journal ----------------------------------------------------

    /// Open a fresh WAL-mode connection to the journal. SQLite connections are
    /// not `Sync`, so each operation opens its own; WAL allows concurrent readers
    /// with a single writer, matching kern's `history.db` idiom.
    pub fn journal(&self) -> Result<Connection> {
        let conn = Connection::open(&self.journal_path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        Ok(conn)
    }

    fn init_journal(&self) -> Result<()> {
        let conn = self.journal()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS message_log (
                 message_id TEXT PRIMARY KEY,
                 sender     TEXT NOT NULL,
                 recipient  TEXT NOT NULL,
                 posted_at  TEXT NOT NULL,
                 expires_unix INTEGER
             );

             CREATE TABLE IF NOT EXISTS claim_events (
                 id        INTEGER PRIMARY KEY AUTOINCREMENT,
                 resource  TEXT NOT NULL,
                 event     TEXT NOT NULL,
                 agent_id  TEXT NOT NULL,
                 fence     INTEGER NOT NULL,
                 at        TEXT NOT NULL
             );

             CREATE TABLE IF NOT EXISTS read_cursors (
                 agent_id TEXT PRIMARY KEY,
                 cursor   TEXT NOT NULL
             );",
        )?;
        Ok(())
    }
}

// --- JSON blob helpers for LMDB values ------------------------------------

/// Encode a record as JSON bytes for storage in an LMDB value slot.
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(value)?)
}

/// Decode an LMDB JSON value blob back into a record.
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    Ok(serde_json::from_slice(bytes)?)
}
