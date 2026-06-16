//! Dynamic tool registry: a live, mutable table of MCP tool descriptors.
//!
//! The 8 hub (mesh) verbs are pre-registered at startup and never removed.
//! External callers may register and unregister additional tools via the
//! `hub_register_tool` and `hub_unregister_tool` MCP verbs. Every mutation
//! sends a token on a `tokio::sync::broadcast` channel; a background task
//! drains the channel and emits a `notifications/tools/list_changed`
//! JSON-RPC notification on stdout (coalesced: one notification per burst).

use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

/// One entry in the registry.
#[derive(Clone, Debug)]
pub struct ToolEntry {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

impl ToolEntry {
    pub fn to_json(&self) -> Value {
        json!({
            "name": self.name,
            "description": self.description,
            "inputSchema": self.input_schema,
        })
    }
}

/// The dynamic tool registry. Clone freely — the inner state is `Arc`-shared.
#[derive(Clone)]
pub struct Registry {
    entries: Arc<Mutex<Vec<ToolEntry>>>,
    tx: broadcast::Sender<()>,
}

impl Registry {
    /// Create a new registry and pre-load the 8 hub verbs.
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(64);
        let reg = Self {
            entries: Arc::new(Mutex::new(Vec::new())),
            tx,
        };
        reg.preload_verbs();
        reg
    }

    /// Subscribe to change notifications. Each item signals one or more
    /// mutations; the receiver should drain all pending items and send a
    /// single `notifications/tools/list_changed`.
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.tx.subscribe()
    }

    /// Return the current tool list sorted by name.
    pub fn list(&self) -> Vec<Value> {
        let entries = self.entries.lock().unwrap();
        let mut tools: Vec<_> = entries.iter().map(|e| e.to_json()).collect();
        tools.sort_by(|a, b| {
            a["name"]
                .as_str()
                .unwrap_or("")
                .cmp(b["name"].as_str().unwrap_or(""))
        });
        tools
    }

    /// Register a new tool or replace an existing one with the same name.
    /// Sends a change notification. Returns `true` if a new entry was added,
    /// `false` if an existing entry was replaced.
    pub fn register(&self, name: String, description: String, input_schema: Value) -> bool {
        let entry = ToolEntry { name: name.clone(), description, input_schema };
        let is_new = {
            let mut entries = self.entries.lock().unwrap();
            if let Some(pos) = entries.iter().position(|e| e.name == name) {
                entries[pos] = entry;
                false
            } else {
                entries.push(entry);
                true
            }
        };
        let _ = self.tx.send(());
        is_new
    }

    /// Unregister a tool by name. The 8 built-in verbs cannot be removed.
    /// Returns `true` if the tool was found and removed.
    pub fn unregister(&self, name: &str) -> bool {
        if BUILTIN_VERBS.contains(&name) {
            return false;
        }
        let removed = {
            let mut entries = self.entries.lock().unwrap();
            let before = entries.len();
            entries.retain(|e| e.name != name);
            entries.len() < before
        };
        if removed {
            let _ = self.tx.send(());
        }
        removed
    }

    /// Pre-load the built-in hub verbs (8 mesh + 11 board + 2 registry + 3 mine tools).
    fn preload_verbs(&self) {
        let verbs: &[(&str, &str, Value)] = &[
            // 8 mesh verbs
            ("register", "Announce presence and refresh liveness (heartbeat).", json!({
                "type":"object","required":["agent_id","branch","prompt_ptr"],
                "properties":{"agent_id":{"type":"string"},"branch":{"type":"string"},"prompt_ptr":{"type":"string"},"role":{"type":"string"},"ttl_seconds":{"type":"integer"}}
            })),
            ("roster", "List known agents and their liveness.", json!({
                "type":"object","required":["agent_id"],
                "properties":{"agent_id":{"type":"string"},"include_stale":{"type":"boolean"}}
            })),
            ("claim", "Atomically acquire (or queue for) a resource lock.", json!({
                "type":"object","required":["agent_id","resource"],
                "properties":{"agent_id":{"type":"string"},"resource":{"type":"string"},"mode":{"type":"string","enum":["exclusive","shared"]},"lease_seconds":{"type":"integer"},"wait":{"type":"string","enum":["no_wait","queue"]},"note":{"type":"string"}}
            })),
            ("release", "Relinquish a held claim or cancel a queued ticket.", json!({
                "type":"object","required":["agent_id","claim_id","resource"],
                "properties":{"agent_id":{"type":"string"},"claim_id":{"type":"string"},"resource":{"type":"string"}}
            })),
            ("claims", "Inspect current locks and queues.", json!({
                "type":"object","required":["agent_id"],
                "properties":{"agent_id":{"type":"string"},"resource":{"type":"string"}}
            })),
            ("post", "Send a durable message (agent_id, * broadcast, or topic:<name>).", json!({
                "type":"object","required":["agent_id","to","body"],
                "properties":{"agent_id":{"type":"string"},"to":{"type":"string"},"subject":{"type":"string"},"body":{"type":"string"},"reply_to":{"type":"string"},"ttl_seconds":{"type":"integer"}}
            })),
            ("inbox", "Peek pending messages without advancing the cursor.", json!({
                "type":"object","required":["agent_id"],
                "properties":{"agent_id":{"type":"string"},"since":{"type":"string"},"topics":{"type":"array","items":{"type":"string"}},"limit":{"type":"integer"}}
            })),
            ("read", "Advance the read cursor (acknowledge consumption).", json!({
                "type":"object","required":["agent_id","up_to"],
                "properties":{"agent_id":{"type":"string"},"up_to":{"type":"string"}}
            })),
            // 2 registry management verbs
            ("hub_register_tool", "Register a new MCP tool in the hub registry; emits tools/list_changed.", json!({
                "type":"object","required":["name","description","input_schema"],
                "properties":{"name":{"type":"string"},"description":{"type":"string"},"input_schema":{"type":"object"}}
            })),
            ("hub_unregister_tool", "Remove a registered MCP tool from the hub registry; the 8 built-in verbs cannot be removed.", json!({
                "type":"object","required":["name"],
                "properties":{"name":{"type":"string"}}
            })),
            // 11 board verbs
            ("project_resolve", "Get-or-create a board project by name (board-per-cwd).", json!({
                "type":"object","required":["name"],
                "properties":{"name":{"type":"string"}}
            })),
            ("project_list", "List all board projects.", json!({
                "type":"object","properties":{}
            })),
            ("board_get", "Read a project's columns and cards, grouped left-to-right with comment counts.", json!({
                "type":"object","required":["projectId"],
                "properties":{"projectId":{"type":"string"}}
            })),
            ("column_create", "Create a column (lifecycle lane) in a project.", json!({
                "type":"object","required":["projectId","name"],
                "properties":{"projectId":{"type":"string"},"name":{"type":"string"}}
            })),
            ("column_delete", "Delete a column and cascade to its cards and comments.", json!({
                "type":"object","required":["id"],
                "properties":{"id":{"type":"string"}}
            })),
            ("card_create", "Create a card in a column.", json!({
                "type":"object","required":["columnId","title"],
                "properties":{"columnId":{"type":"string"},"title":{"type":"string"},"body":{"type":"string"}}
            })),
            ("card_update", "Update a card's title and/or body.", json!({
                "type":"object","required":["id"],
                "properties":{"id":{"type":"string"},"title":{"type":"string"},"body":{"type":"string"}}
            })),
            ("card_move", "Move a card to a column at a 0-based index (reorders the destination).", json!({
                "type":"object","required":["id","toColumnId"],
                "properties":{"id":{"type":"string"},"toColumnId":{"type":"string"},"newIndex":{"type":"integer"}}
            })),
            ("card_delete", "Delete a card and its comments.", json!({
                "type":"object","required":["id"],
                "properties":{"id":{"type":"string"}}
            })),
            ("comment_add", "Add a comment to a card.", json!({
                "type":"object","required":["cardId","author","body"],
                "properties":{"cardId":{"type":"string"},"author":{"type":"string"},"body":{"type":"string"}}
            })),
            ("comment_list", "List a card's comments oldest-first.", json!({
                "type":"object","required":["cardId"],
                "properties":{"cardId":{"type":"string"}}
            })),
            // 3 mine verbs
            ("hub_mine_list", "List the mine catalog (unregistered agents and skills) with install status.", json!({
                "type":"object","properties":{}
            })),
            ("hub_mine_install", "Install a mine catalog item into the project's .claude/ and record it in install.toml.", json!({
                "type":"object","required":["type","name"],
                "properties":{"type":{"type":"string","enum":["skill","agent"]},"name":{"type":"string"}}
            })),
            ("hub_mine_restore", "Idempotently restore every install.toml-listed item missing from the project's .claude/.", json!({
                "type":"object","properties":{}
            })),
        ];

        let mut entries = self.entries.lock().unwrap();
        for (name, desc, schema) in verbs {
            entries.push(ToolEntry {
                name: name.to_string(),
                description: desc.to_string(),
                input_schema: schema.clone(),
            });
        }
    }
}

/// The 8 original hub verbs — these cannot be unregistered.
pub const BUILTIN_VERBS: [&str; 8] = [
    "register", "roster", "claim", "release", "claims", "post", "inbox", "read",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preloads_twenty_four_tools() {
        let reg = Registry::new();
        assert_eq!(reg.list().len(), 24); // 8 mesh + 11 board + 2 registry + 3 mine tools
    }

    #[test]
    fn list_is_sorted_by_name() {
        let reg = Registry::new();
        let list = reg.list();
        let names: Vec<_> = list.iter().map(|v| v["name"].as_str().unwrap()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn register_and_unregister_tool() {
        let reg = Registry::new();
        let was_new = reg.register("my_tool".into(), "desc".into(), json!({"type":"object"}));
        assert!(was_new);
        assert_eq!(reg.list().len(), 25);
        let removed = reg.unregister("my_tool");
        assert!(removed);
        assert_eq!(reg.list().len(), 24);
    }

    #[test]
    fn cannot_unregister_builtin() {
        let reg = Registry::new();
        let removed = reg.unregister("register");
        assert!(!removed);
        assert_eq!(reg.list().len(), 24);
    }

    #[test]
    fn replace_existing_tool() {
        let reg = Registry::new();
        reg.register("dupe".into(), "v1".into(), json!({}));
        let was_new = reg.register("dupe".into(), "v2".into(), json!({}));
        assert!(!was_new); // replacement, not new
        assert_eq!(reg.list().len(), 25); // count unchanged
    }

    #[test]
    fn change_notification_fires_on_register() {
        let reg = Registry::new();
        let mut rx = reg.subscribe();
        reg.register("ping".into(), "p".into(), json!({}));
        // The sender holds capacity=64; the token should be available now.
        assert!(rx.try_recv().is_ok());
    }
}
