//! MCP server over stdio (SPEC §3.1 L-2, mirroring `kern mcp`).
//!
//! A minimal JSON-RPC 2.0 line loop on stdin/stdout implementing the three MCP
//! methods a client needs: `initialize`, `tools/list`, and `tools/call`. The tool
//! list is exactly the eight verbs (V-1); `tools/call` deserializes arguments into
//! the request types and dispatches to the [`Daemon`].

use std::io::{BufRead, Write};

use serde_json::{json, Value};

use crate::error::{MeshError, Result};
use crate::Daemon;

/// Protocol version advertised in the `initialize` reply.
const PROTOCOL_VERSION: &str = "2024-11-05";

/// The eight verbs (V-1). This list is the single source of the tool surface.
pub const VERBS: [&str; 8] = [
    "register", "roster", "claim", "release", "claims", "post", "inbox", "read",
];

/// Run the stdio MCP loop until EOF on stdin. Blocking; one request per line.
pub fn serve_stdio(daemon: &Daemon) -> Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = handle_line(daemon, &line);
        if let Some(resp) = response {
            writeln!(out, "{}", resp)?;
            out.flush()?;
        }
    }
    Ok(())
}

/// Parse one JSON-RPC line and produce the response line (None for notifications).
fn handle_line(daemon: &Daemon, line: &str) -> Option<String> {
    let req: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => {
            return Some(error_response(Value::Null, -32700, "parse error"));
        }
    };
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let is_notification = req.get("id").is_none();

    let result = dispatch_method(daemon, method, req.get("params"));

    match result {
        Ok(Some(value)) => Some(result_response(id, value)),
        Ok(None) => {
            // No result payload (notification handled): emit nothing.
            if is_notification {
                None
            } else {
                Some(result_response(id, json!({})))
            }
        }
        Err(e) => Some(error_response(id, -32000, &e.to_string())),
    }
}

/// Route a JSON-RPC method to its handler.
fn dispatch_method(daemon: &Daemon, method: &str, params: Option<&Value>) -> Result<Option<Value>> {
    match method {
        "initialize" => Ok(Some(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "mesh", "version": env!("CARGO_PKG_VERSION") }
        }))),
        "notifications/initialized" | "initialized" => Ok(None),
        "ping" => Ok(Some(json!({}))),
        "tools/list" => Ok(Some(tools_list())),
        "tools/call" => Ok(Some(tools_call(daemon, params)?)),
        other => Err(MeshError::UnknownVerb(other.to_string())),
    }
}

/// `tools/list` — advertise exactly the eight verbs with input schemas (V-1).
fn tools_list() -> Value {
    let tools: Vec<Value> = VERBS.iter().map(|name| tool_descriptor(name)).collect();
    json!({ "tools": tools })
}

/// `tools/call` — dispatch a verb call to the daemon and wrap the JSON result
/// in MCP's content envelope.
fn tools_call(daemon: &Daemon, params: Option<&Value>) -> Result<Value> {
    let params = params.ok_or_else(|| MeshError::BadRequest("missing params".into()))?;
    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| MeshError::BadRequest("missing tool name".into()))?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    let result = call_verb(daemon, name, args)?;
    let text = serde_json::to_string(&result)?;
    Ok(json!({
        "content": [ { "type": "text", "text": text } ],
        "structuredContent": result
    }))
}

/// Dispatch a single verb by name to the matching `Daemon` method (V-1/V-3).
pub fn call_verb(daemon: &Daemon, name: &str, args: Value) -> Result<Value> {
    match name {
        "register" => Ok(serde_json::to_value(daemon.register(parse(args)?)?)?),
        "roster" => Ok(serde_json::to_value(daemon.roster(parse(args)?)?)?),
        "claim" => Ok(serde_json::to_value(daemon.claim(parse(args)?)?)?),
        "release" => Ok(serde_json::to_value(daemon.release(parse(args)?)?)?),
        "claims" => Ok(serde_json::to_value(daemon.claims(parse(args)?)?)?),
        "post" => Ok(serde_json::to_value(daemon.post(parse(args)?)?)?),
        "inbox" => Ok(serde_json::to_value(daemon.inbox(parse(args)?)?)?),
        "read" => Ok(serde_json::to_value(daemon.read(parse(args)?)?)?),
        other => Err(MeshError::UnknownVerb(other.to_string())),
    }
}

fn parse<T: serde::de::DeserializeOwned>(args: Value) -> Result<T> {
    serde_json::from_value(args).map_err(MeshError::from)
}

// --- JSON-RPC envelope helpers --------------------------------------------

fn result_response(id: Value, result: Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()
}

fn error_response(id: Value, code: i64, message: &str) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
        .to_string()
}

// --- tool schemas ----------------------------------------------------------

/// JSON-Schema input descriptor for one verb. Schemas are intentionally light
/// (required fields only) since the daemon validates semantically.
fn tool_descriptor(name: &str) -> Value {
    let (description, schema) = match name {
        "register" => (
            "Announce presence and refresh liveness (heartbeat).",
            json!({
                "type": "object",
                "required": ["agent_id", "branch", "prompt_ptr"],
                "properties": {
                    "agent_id": { "type": "string" },
                    "branch": { "type": "string" },
                    "prompt_ptr": { "type": "string" },
                    "role": { "type": "string" },
                    "ttl_seconds": { "type": "integer" }
                }
            }),
        ),
        "roster" => (
            "List known agents and their liveness.",
            json!({
                "type": "object",
                "required": ["agent_id"],
                "properties": {
                    "agent_id": { "type": "string" },
                    "include_stale": { "type": "boolean" }
                }
            }),
        ),
        "claim" => (
            "Atomically acquire (or queue for) a resource lock.",
            json!({
                "type": "object",
                "required": ["agent_id", "resource"],
                "properties": {
                    "agent_id": { "type": "string" },
                    "resource": { "type": "string" },
                    "mode": { "type": "string", "enum": ["exclusive", "shared"] },
                    "lease_seconds": { "type": "integer" },
                    "wait": { "type": "string", "enum": ["no_wait", "queue"] },
                    "note": { "type": "string" }
                }
            }),
        ),
        "release" => (
            "Relinquish a held claim or cancel a queued ticket.",
            json!({
                "type": "object",
                "required": ["agent_id", "claim_id", "resource"],
                "properties": {
                    "agent_id": { "type": "string" },
                    "claim_id": { "type": "string" },
                    "resource": { "type": "string" }
                }
            }),
        ),
        "claims" => (
            "Inspect current locks and queues.",
            json!({
                "type": "object",
                "required": ["agent_id"],
                "properties": {
                    "agent_id": { "type": "string" },
                    "resource": { "type": "string" }
                }
            }),
        ),
        "post" => (
            "Send a durable message (agent_id, * broadcast, or topic:<name>).",
            json!({
                "type": "object",
                "required": ["agent_id", "to", "body"],
                "properties": {
                    "agent_id": { "type": "string" },
                    "to": { "type": "string" },
                    "subject": { "type": "string" },
                    "body": { "type": "string" },
                    "reply_to": { "type": "string" },
                    "ttl_seconds": { "type": "integer" }
                }
            }),
        ),
        "inbox" => (
            "Peek pending messages without advancing the cursor.",
            json!({
                "type": "object",
                "required": ["agent_id"],
                "properties": {
                    "agent_id": { "type": "string" },
                    "since": { "type": "string" },
                    "topics": { "type": "array", "items": { "type": "string" } },
                    "limit": { "type": "integer" }
                }
            }),
        ),
        "read" => (
            "Advance the read cursor (acknowledge consumption).",
            json!({
                "type": "object",
                "required": ["agent_id", "up_to"],
                "properties": {
                    "agent_id": { "type": "string" },
                    "up_to": { "type": "string" }
                }
            }),
        ),
        _ => ("", json!({ "type": "object" })),
    };
    json!({
        "name": name,
        "description": description,
        "inputSchema": schema
    })
}
