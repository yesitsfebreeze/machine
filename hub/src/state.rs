//! Persistent state: JSON schema, load, atomic save, and the mkdir-as-mutex lock.
//!
//! State is a single JSON file under a repo-scoped, gitignored `.mesh/`
//! directory (kept for data continuity with the prior `mesh.mjs` daemon).
//! Cross-process atomicity comes from an OS-atomic lock directory held around
//! every mutating op, plus an atomic rename on write.

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::{Duration, Instant};

pub const STATE_FILE: &str = "state.json";
pub const LOCK_DIR: &str = ".lock";

const LOCK_STEAL_MS: u128 = 5000;
const LOCK_SPIN_MS: u64 = 2;

/// A roster entry: one registered agent and its liveness window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RosterEntry {
    pub agent_id: String,
    pub branch: String,
    pub prompt_ptr: String,
    #[serde(default)]
    pub role: Option<String>,
    pub registered_at: i64,
    pub last_seen: i64,
    pub expires_at: i64,
    pub epoch: i64,
}

/// One current holder of a claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Holder {
    pub agent_id: String,
    pub claim_id: String,
    pub lease_expires_at: i64,
    #[serde(default)]
    pub note: Option<String>,
}

/// One queued ticket waiting on a claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticket {
    pub agent_id: String,
    pub claim_id: String,
    pub mode: String,
    pub lease_seconds: i64,
    #[serde(default)]
    pub note: Option<String>,
}

/// A claim record for a single resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimRecord {
    pub resource: String,
    pub mode: String,
    pub holders: Vec<Holder>,
    pub queue: Vec<Ticket>,
    pub fence: i64,
}

/// A durable message record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub message_id: String,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub subject: Option<String>,
    pub body: String,
    #[serde(default)]
    pub reply_to: Option<String>,
    pub posted_at: i64,
    #[serde(default)]
    pub expires_at: Option<i64>,
}

/// One row of the delivery log (ordered by message_id / ULID).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub message_id: String,
    pub sender: String,
    pub recipient: String,
    pub posted_at: String,
    #[serde(default)]
    pub expires_unix: Option<i64>,
}

/// A claim lifecycle event (ISO-timestamped).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub resource: String,
    pub event: String,
    pub agent_id: String,
    pub fence: i64,
    pub at: String,
}

/// The full coordination state. Top-level keys are always present even when
/// empty (parity with the mesh.mjs JSON shape).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct State {
    #[serde(default)]
    pub roster: BTreeMap<String, RosterEntry>,
    #[serde(default)]
    pub claims: BTreeMap<String, ClaimRecord>,
    #[serde(default)]
    pub messages: BTreeMap<String, Message>,
    #[serde(default)]
    pub log: Vec<LogEntry>,
    #[serde(default)]
    pub cursors: BTreeMap<String, String>,
    #[serde(default)]
    pub events: Vec<Event>,
    #[serde(default)]
    pub fence_floor: BTreeMap<String, i64>,
}

/// The on-disk store: data dir plus derived paths.
pub struct Store {
    pub dir: PathBuf,
    pub state_path: PathBuf,
    pub lock_path: PathBuf,
}

impl Store {
    pub fn new(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir)?;
        let state_path = dir.join(STATE_FILE);
        let lock_path = dir.join(LOCK_DIR);
        Ok(Store {
            dir,
            state_path,
            lock_path,
        })
    }

    /// Load state, defaulting any missing top-level keys. A missing or corrupt
    /// file yields a fresh empty state.
    pub fn load(&self) -> State {
        match fs::read_to_string(&self.state_path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => State::default(),
        }
    }

    /// Persist state via temp file + atomic rename.
    pub fn save(&self, state: &State) -> Result<()> {
        let tmp = self
            .dir
            .join(format!("{STATE_FILE}.tmp.{}", std::process::id()));
        let bytes = serde_json::to_string(state)?;
        fs::write(&tmp, bytes)?;
        fs::rename(&tmp, &self.state_path)?;
        Ok(())
    }

    /// Acquire the OS-atomic mutex (mkdir succeeds for exactly one process).
    /// Spin with a short sleep; steal a stale lock so a crashed holder cannot
    /// wedge the daemon.
    fn lock(&self) {
        let start = Instant::now();
        let mut stole = false;
        loop {
            match fs::create_dir(&self.lock_path) {
                Ok(()) => return,
                Err(_) => {
                    if !stole && start.elapsed().as_millis() > LOCK_STEAL_MS {
                        // Assume the holder died mid-op; reclaim and retry once.
                        let _ = fs::remove_dir(&self.lock_path);
                        stole = true;
                    }
                    sleep(Duration::from_millis(LOCK_SPIN_MS));
                }
            }
        }
    }

    fn unlock(&self) {
        let _ = fs::remove_dir(&self.lock_path);
    }

    /// Run `f(&mut state)` under the lock; persist when it returns `mutated = true`.
    pub fn txn<T>(&self, f: impl FnOnce(&mut State) -> Result<(bool, T)>) -> Result<T> {
        self.lock();
        let result = (|| {
            let mut state = self.load();
            let (mutated, value) = f(&mut state)?;
            if mutated {
                self.save(&state)?;
            }
            Ok(value)
        })();
        self.unlock();
        result
    }
}
