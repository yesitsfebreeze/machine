//! Stdio MCP server wiring: read JSON-RPC lines from stdin, dispatch to hub
//! verbs and registry operations, write responses and notifications to stdout.
//!
//! The notification pump runs as a concurrent tokio task: it drains the
//! registry's broadcast channel and emits a single
//! `notifications/tools/list_changed` JSON-RPC notification per burst.

use crate::board::Board;
use crate::error::{HubError, Result};
use crate::mesh::Mesh;
use crate::mine::{self, MineType};
use crate::registry::Registry;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex as AsyncMutex;

pub const PROTOCOL_VERSION: &str = "2024-11-05";
pub const SERVER_NAME: &str = "hub";
pub const SERVER_VERSION: &str = "0.8.0";

// ---- dispatch ---------------------------------------------------------------

/// Invoke a mesh verb by name with its arguments.
fn invoke_mesh_verb(mesh: &Mesh, name: &str, args: &Value) -> Result<Value> {
    match name {
        "register" => mesh.register(args),
        "roster" => mesh.roster(args),
        "claim" => mesh.claim(args),
        "release" => mesh.release(args),
        "claims" => mesh.claims(args),
        "post" => mesh.post(args),
        "inbox" => mesh.inbox(args),
        "read" => mesh.read(args),
        _ => Err(HubError::new(format!("unknown mesh verb '{name}'"))),
    }
}

/// Dispatch a JSON-RPC method. `Ok(None)` means a notification with no reply.
fn dispatch(
    mesh: &Mesh,
    board: &Board,
    registry: &Registry,
    plugin_root: Option<&Path>,
    project_cwd: &Path,
    method: &str,
    params: Option<&Value>,
) -> Result<Option<Value>> {
    match method {
        "initialize" => Ok(Some(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": { "listChanged": true } },
            "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION },
        }))),
        "notifications/initialized" | "initialized" => Ok(None),
        "ping" => Ok(Some(json!({}))),
        "tools/list" => Ok(Some(json!({ "tools": registry.list() }))),
        "tools/call" => {
            let params = params.ok_or_else(|| HubError::new("missing params"))?;
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));

            let result = match name {
                // Registry management verbs
                "hub_register_tool" => {
                    let n = args
                        .get("name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| HubError::new("name is required"))?
                        .to_string();
                    let d = args
                        .get("description")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| HubError::new("description is required"))?
                        .to_string();
                    let s = args
                        .get("input_schema")
                        .cloned()
                        .ok_or_else(|| HubError::new("input_schema is required"))?;
                    let is_new = registry.register(n.clone(), d, s);
                    json!({ "registered": n, "is_new": is_new })
                }
                "hub_unregister_tool" => {
                    let n = args
                        .get("name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| HubError::new("name is required"))?;
                    let removed = registry.unregister(n);
                    json!({ "name": n, "removed": removed })
                }
                // All 8 mesh verbs
                verb if crate::registry::BUILTIN_VERBS.contains(&verb) => {
                    invoke_mesh_verb(mesh, verb, &args)?
                }
                // All 21 board verbs
                verb if crate::board::BOARD_VERBS.contains(&verb) => board.invoke(verb, &args)?,
                "hub_mine_list" => {
                    let root = plugin_root.ok_or_else(|| {
                        HubError::new("plugin root not resolved — mine catalog unavailable")
                    })?;
                    mine::list_json(root, project_cwd)
                }
                "hub_mine_install" => {
                    let ty = args
                        .get("type")
                        .and_then(|v| v.as_str())
                        .and_then(MineType::parse)
                        .ok_or_else(|| HubError::new("type must be 'skill' or 'agent'"))?;
                    let n = args
                        .get("name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| HubError::new("name is required"))?;
                    let root = plugin_root.ok_or_else(|| {
                        HubError::new("plugin root not resolved — mine catalog unavailable")
                    })?;
                    mine::install_item(root, project_cwd, ty, n)?
                }
                "hub_mine_restore" => {
                    let root = plugin_root.ok_or_else(|| {
                        HubError::new("plugin root not resolved — mine catalog unavailable")
                    })?;
                    mine::mine_restore(root, project_cwd)?
                }
                other => return Err(HubError::new(format!("unknown verb '{other}'"))),
            };

            Ok(Some(json!({
                "content": [{ "type": "text", "text": serde_json::to_string(&result)? }],
                "structuredContent": result,
            })))
        }
        other => Err(HubError::new(format!("unknown method '{other}'"))),
    }
}

/// Handle one JSON-RPC line; return the response line (if any) to write.
#[allow(clippy::too_many_arguments)]
fn handle_line(
    mesh: &Mesh,
    board: &Board,
    registry: &Registry,
    plugin_root: Option<&Path>,
    project_cwd: &Path,
    line: &str,
) -> Option<String> {
    let req: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => {
            return Some(
                json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": { "code": -32700, "message": "parse error" }
                })
                .to_string(),
            );
        }
    };
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let is_notification = req.get("id").is_none();
    let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");
    let params = req.get("params");

    match dispatch(
        mesh,
        board,
        registry,
        plugin_root,
        project_cwd,
        method,
        params,
    ) {
        Ok(None) => {
            if is_notification {
                None
            } else {
                Some(json!({ "jsonrpc": "2.0", "id": id, "result": {} }).to_string())
            }
        }
        Ok(Some(result)) => {
            Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string())
        }
        Err(e) => Some(
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32000, "message": e.to_string() }
            })
            .to_string(),
        ),
    }
}

// ---- notification pump ------------------------------------------------------

/// Spawn the notification pump task. It drains the broadcast receiver and
/// emits a single `notifications/tools/list_changed` per burst to the shared
/// stdout writer.
fn spawn_notification_pump(registry: &Registry, stdout_tx: Arc<AsyncMutex<tokio::io::Stdout>>) {
    let mut rx = registry.subscribe();
    tokio::spawn(async move {
        loop {
            // Wait for the first token.
            if rx.recv().await.is_err() {
                break; // channel closed
            }
            // Drain any additional pending tokens (coalesce burst).
            while rx.try_recv().is_ok() {}

            let notification = json!({
                "jsonrpc": "2.0",
                "method": "notifications/tools/list_changed",
            })
            .to_string();

            let mut out = stdout_tx.lock().await;
            let _ = out.write_all(notification.as_bytes()).await;
            let _ = out.write_all(b"\n").await;
            let _ = out.flush().await;
        }
    });
}

// ---- stdio loop -------------------------------------------------------------

/// Serve the MCP protocol over stdio until EOF.
pub async fn serve(
    mesh: Mesh,
    board: Board,
    registry: Registry,
    plugin_root: Option<PathBuf>,
    project_cwd: PathBuf,
) -> Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = Arc::new(AsyncMutex::new(tokio::io::stdout()));

    spawn_notification_pump(&registry, Arc::clone(&stdout));

    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break; // EOF
        }
        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(resp) = handle_line(
            &mesh,
            &board,
            &registry,
            plugin_root.as_deref(),
            &project_cwd,
            &trimmed,
        ) {
            let mut out = stdout.lock().await;
            out.write_all(resp.as_bytes()).await?;
            out.write_all(b"\n").await?;
            out.flush().await?;
        }
    }
    Ok(())
}

// ---- tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn setup() -> (Mesh, Board, Registry, PathBuf) {
        let mut tmp = std::env::temp_dir();
        tmp.push(format!(
            "hub-srv-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let mesh = Mesh::open(tmp.join(".mesh")).unwrap();
        let board = Board::open(tmp.join(".board"), Arc::new(|| 1_000_000)).unwrap();
        let registry = Registry::new();
        (mesh, board, registry, tmp)
    }

    fn call(m: &Mesh, b: &Board, r: &Registry, cwd: &Path, line: &str) -> Option<String> {
        handle_line(m, b, r, None, cwd, line)
    }

    #[test]
    fn initialize_reports_hub_identity_and_list_changed() {
        let (m, b, r, cwd) = setup();
        let resp = call(
            &m,
            &b,
            &r,
            &cwd,
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#,
        )
        .unwrap();
        let v: Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["result"]["serverInfo"]["name"], "hub");
        assert_eq!(v["result"]["serverInfo"]["version"], "0.8.0");
        assert_eq!(v["result"]["capabilities"]["tools"]["listChanged"], true);
    }

    #[test]
    fn tools_list_has_all_builtin_tools() {
        let (m, b, r, cwd) = setup();
        let resp = call(
            &m,
            &b,
            &r,
            &cwd,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        )
        .unwrap();
        let v: Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["result"]["tools"].as_array().unwrap().len(), 34);
    }

    #[test]
    fn hub_register_tool_adds_entry() {
        let (m, b, r, cwd) = setup();
        let req = json!({
            "jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": {
                "name": "hub_register_tool",
                "arguments": {
                    "name": "my_tool",
                    "description": "a test tool",
                    "input_schema": { "type": "object" }
                }
            }
        })
        .to_string();
        let resp = call(&m, &b, &r, &cwd, &req).unwrap();
        let v: Value = serde_json::from_str(&resp).unwrap();
        // Should succeed (no error key)
        assert!(v.get("error").is_none());

        // tools/list should now have 35 entries
        let resp2 = call(
            &m,
            &b,
            &r,
            &cwd,
            r#"{"jsonrpc":"2.0","id":4,"method":"tools/list"}"#,
        )
        .unwrap();
        let v2: Value = serde_json::from_str(&resp2).unwrap();
        assert_eq!(v2["result"]["tools"].as_array().unwrap().len(), 35);
    }

    #[test]
    fn hub_unregister_tool_removes_entry() {
        let (m, b, r, cwd) = setup();
        r.register("temp".into(), "desc".into(), json!({}));
        assert_eq!(r.list().len(), 35);

        let req = json!({
            "jsonrpc": "2.0", "id": 5, "method": "tools/call",
            "params": { "name": "hub_unregister_tool", "arguments": { "name": "temp" } }
        })
        .to_string();
        let resp = call(&m, &b, &r, &cwd, &req).unwrap();
        let v: Value = serde_json::from_str(&resp).unwrap();
        assert!(v.get("error").is_none());
        assert_eq!(r.list().len(), 34);
    }

    #[test]
    fn notification_yields_no_reply() {
        let (m, b, r, cwd) = setup();
        assert!(call(
            &m,
            &b,
            &r,
            &cwd,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#
        )
        .is_none());
    }

    #[test]
    fn parse_error_returns_minus_32700() {
        let (m, b, r, cwd) = setup();
        let resp = call(&m, &b, &r, &cwd, "not json").unwrap();
        let v: Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["error"]["code"], -32700);
    }

    #[test]
    fn mine_list_via_handle_line_returns_items() {
        let (m, b, r, _cwd) = setup();
        let plugin = tempfile::tempdir().unwrap();
        let proj = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(plugin.path().join("mine/agents")).unwrap();
        std::fs::write(
            plugin.path().join("mine/agents/alpha.md"),
            "---\nname: alpha\ndescription: A.\n---\nbody",
        )
        .unwrap();

        let resp = handle_line(
            &m,
            &b,
            &r,
            Some(plugin.path()),
            proj.path(),
            r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"hub_mine_list","arguments":{}}}"#,
        )
        .unwrap();
        let v: Value = serde_json::from_str(&resp).unwrap();
        let items = v["result"]["structuredContent"]["items"]
            .as_array()
            .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["name"], "alpha");
        assert_eq!(items[0]["installed"], false);
    }

    #[test]
    fn mine_install_via_handle_line_copies_and_flips_installed() {
        let (m, b, r, _cwd) = setup();
        let plugin = tempfile::tempdir().unwrap();
        let proj = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(plugin.path().join("mine/agents")).unwrap();
        std::fs::write(
            plugin.path().join("mine/agents/alpha.md"),
            "---\nname: alpha\ndescription: A.\n---\nbody",
        )
        .unwrap();

        let install = handle_line(
            &m, &b, &r, Some(plugin.path()), proj.path(),
            r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"hub_mine_install","arguments":{"type":"agent","name":"alpha"}}}"#,
        ).unwrap();
        let v: Value = serde_json::from_str(&install).unwrap();
        assert_eq!(v["result"]["structuredContent"]["status"], "installed");
        assert!(proj.path().join(".claude/agents/alpha.md").is_file());
        assert!(proj.path().join(".machine/install.toml").is_file());

        let list = handle_line(
            &m, &b, &r, Some(plugin.path()), proj.path(),
            r#"{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"hub_mine_list","arguments":{}}}"#,
        ).unwrap();
        let lv: Value = serde_json::from_str(&list).unwrap();
        let items = lv["result"]["structuredContent"]["items"]
            .as_array()
            .unwrap();
        assert_eq!(items[0]["installed"], true);
    }
}
