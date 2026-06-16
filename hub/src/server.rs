//! Stdio MCP server wiring: read JSON-RPC lines from stdin, dispatch to the
//! eight hub verbs, write responses to stdout.

use crate::error::{HubError, Result};
use crate::mesh::Mesh;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub const PROTOCOL_VERSION: &str = "2024-11-05";
pub const SERVER_NAME: &str = "hub";
pub const SERVER_VERSION: &str = "0.4.0";

/// The eight verbs (single source of the tool surface).
pub const VERBS: [&str; 8] = [
    "register", "roster", "claim", "release", "claims", "post", "inbox", "read",
];

/// Tool descriptions + JSON-schema for tools/list.
fn tools_list() -> Value {
    let tools = vec![
        tool("register", "Announce presence and refresh liveness (heartbeat).", json!({
            "type":"object","required":["agent_id","branch","prompt_ptr"],
            "properties":{"agent_id":{"type":"string"},"branch":{"type":"string"},"prompt_ptr":{"type":"string"},"role":{"type":"string"},"ttl_seconds":{"type":"integer"}}
        })),
        tool("roster", "List known agents and their liveness.", json!({
            "type":"object","required":["agent_id"],
            "properties":{"agent_id":{"type":"string"},"include_stale":{"type":"boolean"}}
        })),
        tool("claim", "Atomically acquire (or queue for) a resource lock.", json!({
            "type":"object","required":["agent_id","resource"],
            "properties":{"agent_id":{"type":"string"},"resource":{"type":"string"},"mode":{"type":"string","enum":["exclusive","shared"]},"lease_seconds":{"type":"integer"},"wait":{"type":"string","enum":["no_wait","queue"]},"note":{"type":"string"}}
        })),
        tool("release", "Relinquish a held claim or cancel a queued ticket.", json!({
            "type":"object","required":["agent_id","claim_id","resource"],
            "properties":{"agent_id":{"type":"string"},"claim_id":{"type":"string"},"resource":{"type":"string"}}
        })),
        tool("claims", "Inspect current locks and queues.", json!({
            "type":"object","required":["agent_id"],
            "properties":{"agent_id":{"type":"string"},"resource":{"type":"string"}}
        })),
        tool("post", "Send a durable message (agent_id, * broadcast, or topic:<name>).", json!({
            "type":"object","required":["agent_id","to","body"],
            "properties":{"agent_id":{"type":"string"},"to":{"type":"string"},"subject":{"type":"string"},"body":{"type":"string"},"reply_to":{"type":"string"},"ttl_seconds":{"type":"integer"}}
        })),
        tool("inbox", "Peek pending messages without advancing the cursor.", json!({
            "type":"object","required":["agent_id"],
            "properties":{"agent_id":{"type":"string"},"since":{"type":"string"},"topics":{"type":"array","items":{"type":"string"}},"limit":{"type":"integer"}}
        })),
        tool("read", "Advance the read cursor (acknowledge consumption).", json!({
            "type":"object","required":["agent_id","up_to"],
            "properties":{"agent_id":{"type":"string"},"up_to":{"type":"string"}}
        })),
    ];
    json!({ "tools": tools })
}

fn tool(name: &str, description: &str, schema: Value) -> Value {
    json!({ "name": name, "description": description, "inputSchema": schema })
}

/// Invoke a verb by name with its arguments.
fn invoke(mesh: &Mesh, name: &str, args: &Value) -> Result<Value> {
    match name {
        "register" => mesh.register(args),
        "roster" => mesh.roster(args),
        "claim" => mesh.claim(args),
        "release" => mesh.release(args),
        "claims" => mesh.claims(args),
        "post" => mesh.post(args),
        "inbox" => mesh.inbox(args),
        "read" => mesh.read(args),
        _ => Err(HubError::new(format!("unknown verb '{name}'"))),
    }
}

/// Dispatch a JSON-RPC method. `Ok(None)` means a notification with no reply.
fn dispatch(mesh: &Mesh, method: &str, params: Option<&Value>) -> Result<Option<Value>> {
    match method {
        "initialize" => Ok(Some(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION },
        }))),
        "notifications/initialized" | "initialized" => Ok(None),
        "ping" => Ok(Some(json!({}))),
        "tools/list" => Ok(Some(tools_list())),
        "tools/call" => {
            let params = params.ok_or_else(|| HubError::new("missing params"))?;
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if !VERBS.contains(&name) {
                return Err(HubError::new(format!("unknown verb '{name}'")));
            }
            let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
            let result = invoke(mesh, name, &args)?;
            Ok(Some(json!({
                "content": [{ "type": "text", "text": serde_json::to_string(&result)? }],
                "structuredContent": result,
            })))
        }
        other => Err(HubError::new(format!("unknown method '{other}'"))),
    }
}

/// Handle one JSON-RPC line; return the response line (if any) to write.
fn handle_line(mesh: &Mesh, line: &str) -> Option<String> {
    let req: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => {
            return Some(
                json!({ "jsonrpc": "2.0", "id": Value::Null, "error": { "code": -32700, "message": "parse error" } })
                    .to_string(),
            );
        }
    };
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let is_notification = req.get("id").is_none();
    let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");
    let params = req.get("params");

    match dispatch(mesh, method, params) {
        Ok(None) => {
            if is_notification {
                None
            } else {
                Some(json!({ "jsonrpc": "2.0", "id": id, "result": {} }).to_string())
            }
        }
        Ok(Some(result)) => Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()),
        Err(e) => Some(
            json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32000, "message": e.to_string() } })
                .to_string(),
        ),
    }
}

/// Serve the MCP protocol over stdio until EOF.
pub async fn serve(mesh: Mesh) -> Result<()> {
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break; // EOF
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(resp) = handle_line(&mesh, trimmed) {
            stdout.write_all(resp.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn mesh() -> Mesh {
        let mut tmp = std::env::temp_dir();
        tmp.push(format!(
            "hub-srv-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        Mesh::open(tmp.join(".mesh")).unwrap()
    }

    #[test]
    fn initialize_reports_hub_identity() {
        let m = mesh();
        let resp = handle_line(&m, r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#).unwrap();
        let v: Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["result"]["serverInfo"]["name"], "hub");
        assert_eq!(v["result"]["serverInfo"]["version"], "0.4.0");
    }

    #[test]
    fn tools_list_has_eight_verbs() {
        let m = mesh();
        let resp = handle_line(&m, r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#).unwrap();
        let v: Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["result"]["tools"].as_array().unwrap().len(), 8);
    }

    #[test]
    fn notification_yields_no_reply() {
        let m = mesh();
        assert!(handle_line(&m, r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).is_none());
    }

    #[test]
    fn parse_error_returns_minus_32700() {
        let m = mesh();
        let resp = handle_line(&m, "not json").unwrap();
        let v: Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["error"]["code"], -32700);
    }
}
