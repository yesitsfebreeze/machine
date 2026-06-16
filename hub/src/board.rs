//! Board state: projects -> columns -> cards -> comments.
//!
//! Schema is byte-compatible with `.board/state.json` written by board.mjs.
//! The Store mirrors hub/src/state.rs in locking strategy (mkdir-as-mutex,
//! atomic rename on write, 5-second lock-steal timeout).

use crate::error::{HubError, Result};
use crate::mesh::iso_from_unix;
use crate::ulid_gen::UlidGen;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};

pub const STATE_FILE: &str = "state.json";
pub const LOCK_DIR: &str = ".lock";
const LOCK_STEAL_MS: u128 = 5000;
const LOCK_SPIN_MS: u64 = 2;

// ---- data model ------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub id: String,
    #[serde(rename = "projectId")]
    pub project_id: String,
    pub name: String,
    pub sort: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: String,
    #[serde(rename = "columnId")]
    pub column_id: String,
    pub title: String,
    #[serde(default)]
    pub body: String,
    pub sort: i64,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    #[serde(rename = "cardId")]
    pub card_id: String,
    pub author: String,
    pub body: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BoardState {
    #[serde(default)]
    pub projects: BTreeMap<String, Project>,
    #[serde(default)]
    pub columns: BTreeMap<String, Column>,
    #[serde(default)]
    pub cards: BTreeMap<String, Card>,
    #[serde(default)]
    pub comments: BTreeMap<String, Comment>,
    #[serde(default)]
    pub rev: u64,
}

// ---- on-disk store ---------------------------------------------------------

pub struct BoardStore {
    pub dir: PathBuf,
    pub state_path: PathBuf,
    pub lock_path: PathBuf,
}

impl BoardStore {
    pub fn new(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir)?;
        let state_path = dir.join(STATE_FILE);
        let lock_path = dir.join(LOCK_DIR);
        Ok(BoardStore { dir, state_path, lock_path })
    }

    pub fn load(&self) -> BoardState {
        match fs::read_to_string(&self.state_path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => BoardState::default(),
        }
    }

    pub fn save(&self, state: &BoardState) -> Result<()> {
        let tmp = self.dir.join(format!("{STATE_FILE}.tmp.{}", std::process::id()));
        let bytes = serde_json::to_string(state)?;
        fs::write(&tmp, bytes)?;
        fs::rename(&tmp, &self.state_path)?;
        Ok(())
    }

    fn lock(&self) {
        let start = Instant::now();
        let mut stole = false;
        loop {
            match fs::create_dir(&self.lock_path) {
                Ok(()) => return,
                Err(_) => {
                    if !stole && start.elapsed().as_millis() > LOCK_STEAL_MS {
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
    pub fn txn<T>(&self, f: impl FnOnce(&mut BoardState) -> Result<(bool, T)>) -> Result<T> {
        self.lock();
        let result = (|| {
            let mut state = self.load();
            let (mutated, value) = f(&mut state)?;
            if mutated {
                state.rev += 1;
                self.save(&state)?;
            }
            Ok(value)
        })();
        self.unlock();
        result
    }
}

// ---- Board verb implementations --------------------------------------------

/// A clock returning Unix seconds. Injectable for tests.
pub type Clock = Arc<dyn Fn() -> i64 + Send + Sync>;

pub fn system_clock() -> Clock {
    Arc::new(|| {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    })
}

/// Board verb dispatcher. Holds references to the store and clock.
pub struct Board {
    pub store: BoardStore,
    pub clock: Clock,
    pub ulid: UlidGen,
}

impl Board {
    pub fn open(dir: impl AsRef<Path>, clock: Clock) -> Result<Self> {
        let store = BoardStore::new(dir)?;
        Ok(Board { store, clock, ulid: UlidGen::new() })
    }

    fn now_iso(&self) -> String {
        iso_from_unix((self.clock)())
    }

    fn new_id(&self) -> String {
        self.ulid.next()
    }

    // ---- projects ----------------------------------------------------------

    pub fn project_resolve(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let name = args.get("name").and_then(|v| v.as_str())
            .ok_or_else(|| HubError::new("name is required"))?
            .to_string();
        self.store.txn(|state| {
            if let Some(p) = state.projects.values().find(|p| p.name == name).cloned() {
                return Ok((false, serde_json::json!({ "project": p })));
            }
            let id = self.new_id();
            let project = Project { id: id.clone(), name, created_at: self.now_iso() };
            state.projects.insert(id, project.clone());
            Ok((true, serde_json::json!({ "project": project })))
        })
    }

    pub fn project_list(&self, _args: &serde_json::Value) -> Result<serde_json::Value> {
        self.store.txn(|state| {
            let mut projects: Vec<Project> = state.projects.values().cloned().collect();
            projects.sort_by(|a, b| a.created_at.cmp(&b.created_at));
            Ok((false, serde_json::json!({ "projects": projects })))
        })
    }

    // ---- board read --------------------------------------------------------

    pub fn board_get(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let project_id = args.get("projectId").and_then(|v| v.as_str())
            .ok_or_else(|| HubError::new("projectId is required"))?
            .to_string();
        self.store.txn(|state| {
            let project = state.projects.get(&project_id).cloned()
                .ok_or_else(|| HubError::new(format!("unknown project '{project_id}'")))?;

            // Build comment counts
            let mut comment_counts: BTreeMap<String, u64> = BTreeMap::new();
            for c in state.comments.values() {
                *comment_counts.entry(c.card_id.clone()).or_insert(0) += 1;
            }

            // Build columns with cards
            let mut columns: Vec<_> = state.columns.values()
                .filter(|col| col.project_id == project_id)
                .cloned()
                .collect();
            columns.sort_by(|a, b| a.sort.cmp(&b.sort).then(a.id.cmp(&b.id)));

            let columns_with_cards: Vec<serde_json::Value> = columns.into_iter().map(|col| {
                let mut cards: Vec<_> = state.cards.values()
                    .filter(|k| k.column_id == col.id)
                    .cloned()
                    .collect();
                cards.sort_by(|a, b| a.sort.cmp(&b.sort).then(a.id.cmp(&b.id)));
                let cards_with_counts: Vec<serde_json::Value> = cards.into_iter().map(|card| {
                    let count = comment_counts.get(&card.id).copied().unwrap_or(0);
                    let mut v = serde_json::to_value(&card).unwrap();
                    v.as_object_mut().unwrap().insert("commentCount".to_string(), serde_json::json!(count));
                    v
                }).collect();
                let mut v = serde_json::to_value(&col).unwrap();
                v.as_object_mut().unwrap().insert("cards".to_string(), serde_json::json!(cards_with_counts));
                v
            }).collect();

            Ok((false, serde_json::json!({ "project": project, "columns": columns_with_cards })))
        })
    }

    // ---- columns -----------------------------------------------------------

    pub fn column_create(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let project_id = args.get("projectId").and_then(|v| v.as_str())
            .ok_or_else(|| HubError::new("projectId is required"))?
            .to_string();
        let name = args.get("name").and_then(|v| v.as_str())
            .ok_or_else(|| HubError::new("name is required"))?
            .to_string();
        self.store.txn(|state| {
            if !state.projects.contains_key(&project_id) {
                return Err(HubError::new(format!("unknown project '{project_id}'")));
            }
            let max_sort = state.columns.values()
                .filter(|c| c.project_id == project_id)
                .map(|c| c.sort)
                .max()
                .map(|s| s + 1)
                .unwrap_or(0);
            let id = self.new_id();
            let column = Column { id: id.clone(), project_id, name, sort: max_sort };
            state.columns.insert(id, column.clone());
            Ok((true, serde_json::json!({ "column": column })))
        })
    }

    pub fn column_delete(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let id = args.get("id").and_then(|v| v.as_str())
            .ok_or_else(|| HubError::new("id is required"))?
            .to_string();
        self.store.txn(|state| {
            if !state.columns.contains_key(&id) {
                return Ok((false, serde_json::json!({ "status": "unknown" })));
            }
            state.columns.remove(&id);
            // Cascade: delete all cards in this column, and their comments
            let card_ids: Vec<String> = state.cards.values()
                .filter(|k| k.column_id == id)
                .map(|k| k.id.clone())
                .collect();
            for card_id in &card_ids {
                state.cards.remove(card_id);
                state.comments.retain(|_, c| &c.card_id != card_id);
            }
            Ok((true, serde_json::json!({ "status": "deleted" })))
        })
    }

    // ---- cards -------------------------------------------------------------

    pub fn card_create(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let column_id = args.get("columnId").and_then(|v| v.as_str())
            .ok_or_else(|| HubError::new("columnId is required"))?
            .to_string();
        let title = args.get("title").and_then(|v| v.as_str())
            .ok_or_else(|| HubError::new("title is required"))?
            .to_string();
        let body = args.get("body").and_then(|v| v.as_str()).unwrap_or("").to_string();
        self.store.txn(|state| {
            if !state.columns.contains_key(&column_id) {
                return Err(HubError::new(format!("unknown column '{column_id}'")));
            }
            let max_sort = state.cards.values()
                .filter(|k| k.column_id == column_id)
                .map(|k| k.sort)
                .max()
                .map(|s| s + 1)
                .unwrap_or(0);
            let id = self.new_id();
            let card = Card { id: id.clone(), column_id, title, body, sort: max_sort, created_at: self.now_iso() };
            state.cards.insert(id, card.clone());
            Ok((true, serde_json::json!({ "card": card })))
        })
    }

    pub fn card_update(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let id = args.get("id").and_then(|v| v.as_str())
            .ok_or_else(|| HubError::new("id is required"))?
            .to_string();
        self.store.txn(|state| {
            let card = state.cards.get_mut(&id)
                .ok_or_else(|| HubError::new(format!("unknown card '{id}'")))?;
            if let Some(title) = args.get("title").and_then(|v| v.as_str()) {
                card.title = title.to_string();
            }
            if let Some(body) = args.get("body").and_then(|v| v.as_str()) {
                card.body = body.to_string();
            }
            let card = card.clone();
            Ok((true, serde_json::json!({ "card": card })))
        })
    }

    pub fn card_move(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let id = args.get("id").and_then(|v| v.as_str())
            .ok_or_else(|| HubError::new("id is required"))?
            .to_string();
        let to_column_id = args.get("toColumnId").and_then(|v| v.as_str())
            .ok_or_else(|| HubError::new("toColumnId is required"))?
            .to_string();
        let new_index = args.get("newIndex").and_then(|v| v.as_i64());
        self.store.txn(|state| {
            if !state.cards.contains_key(&id) {
                return Err(HubError::new(format!("unknown card '{id}'")));
            }
            if !state.columns.contains_key(&to_column_id) {
                return Err(HubError::new(format!("unknown column '{to_column_id}'")));
            }
            // Update the card's column
            state.cards.get_mut(&id).unwrap().column_id = to_column_id.clone();
            // Collect siblings (excluding the card itself), sorted by sort then id
            let mut siblings: Vec<_> = state.cards.values()
                .filter(|k| k.column_id == to_column_id && k.id != id)
                .cloned()
                .collect();
            siblings.sort_by(|a, b| a.sort.cmp(&b.sort).then(a.id.cmp(&b.id)));
            // Get the moved card
            let moved_card = state.cards.get(&id).unwrap().clone();
            // Insert at clamped index
            let idx = new_index.unwrap_or(siblings.len() as i64);
            let idx = idx.max(0).min(siblings.len() as i64) as usize;
            siblings.insert(idx, moved_card);
            // Renumber
            for (i, card) in siblings.iter().enumerate() {
                state.cards.get_mut(&card.id).unwrap().sort = i as i64;
            }
            let card = state.cards.get(&id).unwrap().clone();
            Ok((true, serde_json::json!({ "card": card })))
        })
    }

    pub fn card_delete(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let id = args.get("id").and_then(|v| v.as_str())
            .ok_or_else(|| HubError::new("id is required"))?
            .to_string();
        self.store.txn(|state| {
            if !state.cards.contains_key(&id) {
                return Ok((false, serde_json::json!({ "status": "unknown" })));
            }
            state.cards.remove(&id);
            state.comments.retain(|_, c| c.card_id != id);
            Ok((true, serde_json::json!({ "status": "deleted" })))
        })
    }

    // ---- comments ----------------------------------------------------------

    pub fn comment_add(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let card_id = args.get("cardId").and_then(|v| v.as_str())
            .ok_or_else(|| HubError::new("cardId is required"))?
            .to_string();
        let author = args.get("author").and_then(|v| v.as_str())
            .ok_or_else(|| HubError::new("author is required"))?
            .to_string();
        let body = args.get("body").and_then(|v| v.as_str())
            .ok_or_else(|| HubError::new("body is required"))?
            .to_string();
        self.store.txn(|state| {
            if !state.cards.contains_key(&card_id) {
                return Err(HubError::new(format!("unknown card '{card_id}'")));
            }
            let id = self.new_id();
            let comment = Comment { id: id.clone(), card_id, author, body, created_at: self.now_iso() };
            state.comments.insert(id, comment.clone());
            Ok((true, serde_json::json!({ "comment": comment })))
        })
    }

    pub fn comment_list(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let card_id = args.get("cardId").and_then(|v| v.as_str())
            .ok_or_else(|| HubError::new("cardId is required"))?
            .to_string();
        self.store.txn(|state| {
            let mut comments: Vec<_> = state.comments.values()
                .filter(|c| c.card_id == card_id)
                .cloned()
                .collect();
            comments.sort_by(|a, b| a.created_at.cmp(&b.created_at).then(a.id.cmp(&b.id)));
            Ok((false, serde_json::json!({ "comments": comments })))
        })
    }

    /// Dispatch a board verb by name.
    pub fn invoke(&self, name: &str, args: &serde_json::Value) -> Result<serde_json::Value> {
        match name {
            "project_resolve" => self.project_resolve(args),
            "project_list"    => self.project_list(args),
            "board_get"       => self.board_get(args),
            "column_create"   => self.column_create(args),
            "column_delete"   => self.column_delete(args),
            "card_create"     => self.card_create(args),
            "card_update"     => self.card_update(args),
            "card_move"       => self.card_move(args),
            "card_delete"     => self.card_delete(args),
            "comment_add"     => self.comment_add(args),
            "comment_list"    => self.comment_list(args),
            _ => Err(HubError::new(format!("unknown board verb '{name}'"))),
        }
    }
}

pub const BOARD_VERBS: [&str; 11] = [
    "project_resolve", "project_list", "board_get",
    "column_create", "column_delete",
    "card_create", "card_update", "card_move", "card_delete",
    "comment_add", "comment_list",
];

// ---- tests -----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI64, Ordering};
    use std::sync::Arc;

    fn make_board(base: &std::path::Path, t: i64) -> Board {
        let t = Arc::new(AtomicI64::new(t));
        let clock: Clock = {
            let t = Arc::clone(&t);
            Arc::new(move || t.load(Ordering::SeqCst))
        };
        Board::open(base.join(".board"), clock).unwrap()
    }

    fn tmp_dir(suffix: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("hub-board-{}-{}", std::process::id(), suffix));
        p
    }

    #[test]
    fn project_resolve_idempotency() {
        let dir = tmp_dir("proj-resolve");
        let b = make_board(&dir, 1_000_000);
        let r1 = b.project_resolve(&serde_json::json!({"name": "myproject"})).unwrap();
        let r2 = b.project_resolve(&serde_json::json!({"name": "myproject"})).unwrap();
        assert_eq!(r1["project"]["id"], r2["project"]["id"]);
        // rev only increments on first creation
        let state = b.store.load();
        assert_eq!(state.rev, 1);
    }

    #[test]
    fn project_list_ordering() {
        let dir = tmp_dir("proj-list");
        let b = make_board(&dir, 1_000_000);
        b.project_resolve(&serde_json::json!({"name": "alpha"})).unwrap();
        b.project_resolve(&serde_json::json!({"name": "beta"})).unwrap();
        let r = b.project_list(&serde_json::json!({})).unwrap();
        let names: Vec<&str> = r["projects"].as_array().unwrap()
            .iter().map(|p| p["name"].as_str().unwrap()).collect();
        // ordered by createdAt; same timestamp -> id order (ULID time order)
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn board_get_structure() {
        let dir = tmp_dir("board-get");
        let b = make_board(&dir, 1_000_000);
        let pr = b.project_resolve(&serde_json::json!({"name": "p"})).unwrap();
        let pid = pr["project"]["id"].as_str().unwrap().to_string();
        let col = b.column_create(&serde_json::json!({"projectId": pid, "name": "Todo"})).unwrap();
        let cid = col["column"]["id"].as_str().unwrap().to_string();
        b.card_create(&serde_json::json!({"columnId": cid, "title": "Task 1"})).unwrap();
        b.card_create(&serde_json::json!({"columnId": cid, "title": "Task 2"})).unwrap();
        let r = b.board_get(&serde_json::json!({"projectId": pid})).unwrap();
        let cols = r["columns"].as_array().unwrap();
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0]["cards"].as_array().unwrap().len(), 2);
        assert_eq!(cols[0]["cards"][0]["commentCount"], 0);
    }

    #[test]
    fn column_create_sort_ordering() {
        let dir = tmp_dir("col-sort");
        let b = make_board(&dir, 1_000_000);
        let pr = b.project_resolve(&serde_json::json!({"name": "p"})).unwrap();
        let pid = pr["project"]["id"].as_str().unwrap();
        let c0 = b.column_create(&serde_json::json!({"projectId": pid, "name": "A"})).unwrap();
        let c1 = b.column_create(&serde_json::json!({"projectId": pid, "name": "B"})).unwrap();
        let c2 = b.column_create(&serde_json::json!({"projectId": pid, "name": "C"})).unwrap();
        assert_eq!(c0["column"]["sort"], 0);
        assert_eq!(c1["column"]["sort"], 1);
        assert_eq!(c2["column"]["sort"], 2);
    }

    #[test]
    fn column_delete_cascade() {
        let dir = tmp_dir("col-cascade");
        let b = make_board(&dir, 1_000_000);
        let pr = b.project_resolve(&serde_json::json!({"name": "p"})).unwrap();
        let pid = pr["project"]["id"].as_str().unwrap();
        let col = b.column_create(&serde_json::json!({"projectId": pid, "name": "X"})).unwrap();
        let cid = col["column"]["id"].as_str().unwrap();
        let card = b.card_create(&serde_json::json!({"columnId": cid, "title": "T"})).unwrap();
        let kid = card["card"]["id"].as_str().unwrap();
        b.comment_add(&serde_json::json!({"cardId": kid, "author": "a", "body": "b"})).unwrap();
        let r = b.column_delete(&serde_json::json!({"id": cid})).unwrap();
        assert_eq!(r["status"], "deleted");
        let state = b.store.load();
        assert!(state.cards.is_empty());
        assert!(state.comments.is_empty());
    }

    #[test]
    fn card_create_default_body_and_sort() {
        let dir = tmp_dir("card-create");
        let b = make_board(&dir, 1_000_000);
        let pr = b.project_resolve(&serde_json::json!({"name": "p"})).unwrap();
        let pid = pr["project"]["id"].as_str().unwrap();
        let col = b.column_create(&serde_json::json!({"projectId": pid, "name": "C"})).unwrap();
        let cid = col["column"]["id"].as_str().unwrap();
        let r = b.card_create(&serde_json::json!({"columnId": cid, "title": "X"})).unwrap();
        assert_eq!(r["card"]["body"], "");
        assert_eq!(r["card"]["sort"], 0);
        let r2 = b.card_create(&serde_json::json!({"columnId": cid, "title": "Y"})).unwrap();
        assert_eq!(r2["card"]["sort"], 1);
    }

    #[test]
    fn card_update_partial() {
        let dir = tmp_dir("card-update");
        let b = make_board(&dir, 1_000_000);
        let pr = b.project_resolve(&serde_json::json!({"name": "p"})).unwrap();
        let pid = pr["project"]["id"].as_str().unwrap();
        let col = b.column_create(&serde_json::json!({"projectId": pid, "name": "C"})).unwrap();
        let cid = col["column"]["id"].as_str().unwrap();
        let card = b.card_create(&serde_json::json!({"columnId": cid, "title": "Old", "body": "original"})).unwrap();
        let kid = card["card"]["id"].as_str().unwrap();
        // title only
        b.card_update(&serde_json::json!({"id": kid, "title": "New"})).unwrap();
        let state = b.store.load();
        let c = state.cards.get(kid).unwrap();
        assert_eq!(c.title, "New");
        assert_eq!(c.body, "original");
        // body only
        b.card_update(&serde_json::json!({"id": kid, "body": "updated"})).unwrap();
        let state = b.store.load();
        let c = state.cards.get(kid).unwrap();
        assert_eq!(c.title, "New");
        assert_eq!(c.body, "updated");
    }

    #[test]
    fn card_move_across_columns() {
        let dir = tmp_dir("card-move");
        let b = make_board(&dir, 1_000_000);
        let pr = b.project_resolve(&serde_json::json!({"name": "p"})).unwrap();
        let pid = pr["project"]["id"].as_str().unwrap();
        let col1 = b.column_create(&serde_json::json!({"projectId": pid, "name": "A"})).unwrap();
        let col2 = b.column_create(&serde_json::json!({"projectId": pid, "name": "B"})).unwrap();
        let c1id = col1["column"]["id"].as_str().unwrap();
        let c2id = col2["column"]["id"].as_str().unwrap();
        let card = b.card_create(&serde_json::json!({"columnId": c1id, "title": "T"})).unwrap();
        let kid = card["card"]["id"].as_str().unwrap();
        b.card_move(&serde_json::json!({"id": kid, "toColumnId": c2id, "newIndex": 0})).unwrap();
        let state = b.store.load();
        assert_eq!(state.cards.get(kid).unwrap().column_id, c2id);
        assert_eq!(state.cards.get(kid).unwrap().sort, 0);
    }

    #[test]
    fn card_move_within_column() {
        let dir = tmp_dir("card-move-within");
        let b = make_board(&dir, 1_000_000);
        let pr = b.project_resolve(&serde_json::json!({"name": "p"})).unwrap();
        let pid = pr["project"]["id"].as_str().unwrap();
        let col = b.column_create(&serde_json::json!({"projectId": pid, "name": "A"})).unwrap();
        let cid = col["column"]["id"].as_str().unwrap();
        let k0 = b.card_create(&serde_json::json!({"columnId": cid, "title": "First"})).unwrap();
        let k1 = b.card_create(&serde_json::json!({"columnId": cid, "title": "Second"})).unwrap();
        let k2 = b.card_create(&serde_json::json!({"columnId": cid, "title": "Third"})).unwrap();
        let kid0 = k0["card"]["id"].as_str().unwrap();
        let _kid1 = k1["card"]["id"].as_str().unwrap();
        let _kid2 = k2["card"]["id"].as_str().unwrap();
        // Move first card to position 2 (end)
        b.card_move(&serde_json::json!({"id": kid0, "toColumnId": cid, "newIndex": 2})).unwrap();
        let state = b.store.load();
        assert_eq!(state.cards.get(kid0).unwrap().sort, 2);
    }

    #[test]
    fn card_delete_cascade() {
        let dir = tmp_dir("card-del");
        let b = make_board(&dir, 1_000_000);
        let pr = b.project_resolve(&serde_json::json!({"name": "p"})).unwrap();
        let pid = pr["project"]["id"].as_str().unwrap();
        let col = b.column_create(&serde_json::json!({"projectId": pid, "name": "C"})).unwrap();
        let cid = col["column"]["id"].as_str().unwrap();
        let card = b.card_create(&serde_json::json!({"columnId": cid, "title": "T"})).unwrap();
        let kid = card["card"]["id"].as_str().unwrap();
        b.comment_add(&serde_json::json!({"cardId": kid, "author": "a", "body": "b"})).unwrap();
        b.card_delete(&serde_json::json!({"id": kid})).unwrap();
        let state = b.store.load();
        assert!(state.cards.is_empty());
        assert!(state.comments.is_empty());
    }

    #[test]
    fn comment_add_and_list_oldest_first() {
        let dir = tmp_dir("comments");
        let b = make_board(&dir, 1_000_000);
        let pr = b.project_resolve(&serde_json::json!({"name": "p"})).unwrap();
        let pid = pr["project"]["id"].as_str().unwrap();
        let col = b.column_create(&serde_json::json!({"projectId": pid, "name": "C"})).unwrap();
        let cid = col["column"]["id"].as_str().unwrap();
        let card = b.card_create(&serde_json::json!({"columnId": cid, "title": "T"})).unwrap();
        let kid = card["card"]["id"].as_str().unwrap();
        b.comment_add(&serde_json::json!({"cardId": kid, "author": "a", "body": "first"})).unwrap();
        b.comment_add(&serde_json::json!({"cardId": kid, "author": "b", "body": "second"})).unwrap();
        let r = b.comment_list(&serde_json::json!({"cardId": kid})).unwrap();
        let comments = r["comments"].as_array().unwrap();
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0]["body"], "first");
        assert_eq!(comments[1]["body"], "second");
    }

    #[test]
    fn column_delete_unknown_returns_unknown() {
        let dir = tmp_dir("col-unknown");
        let b = make_board(&dir, 1_000_000);
        let r = b.column_delete(&serde_json::json!({"id": "NONEXISTENT"})).unwrap();
        assert_eq!(r["status"], "unknown");
    }

    #[test]
    fn card_delete_unknown_returns_unknown() {
        let dir = tmp_dir("card-unknown");
        let b = make_board(&dir, 1_000_000);
        let r = b.card_delete(&serde_json::json!({"id": "NONEXISTENT"})).unwrap();
        assert_eq!(r["status"], "unknown");
    }
}
