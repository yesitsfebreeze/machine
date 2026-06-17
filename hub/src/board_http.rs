//! HTTP+SSE+WS server for hub serve.
//!
//! Routes:
//!   GET  /           HTML board UI
//!   GET  /health     JSON health check
//!   POST /mcp        JSON-RPC MCP request (returns JSON)
//!   GET  /mcp        SSE stream for MCP notifications (session-keyed)
//!   GET  /ws         WebSocket board+roster live view
//!   POST /api/<verb> REST convenience wrappers for board drag-and-drop

use crate::board::{self, Board};
use crate::error::Result;
use crate::mesh::Mesh;
use crate::mine::{self, HubTomlConfig, MineType};
use crate::registry::Registry;
use crate::server::{PROTOCOL_VERSION, SERVER_NAME, SERVER_VERSION};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

// ---- shared state ----------------------------------------------------------

#[derive(Clone)]
struct HubState {
    mesh: Arc<Mesh>,
    board: Arc<Board>,
    registry: Arc<Registry>,
    /// session id -> sender for SSE outbound JSON-RPC responses
    sessions: Arc<RwLock<HashMap<String, mpsc::Sender<String>>>>,
    /// fires a token on every board mutation (WS push trigger)
    board_tx: broadcast::Sender<()>,
    /// fires a token on every mesh mutation (WS push trigger)
    mesh_tx: broadcast::Sender<()>,
    /// resolved plugin root (contains `mine/`); None disables mine tools
    plugin_root: Option<PathBuf>,
    /// project cwd used as the install/restore target for mine ops
    project_cwd: PathBuf,
    /// hub.toml proxy config (load-only; reserved for a future proxy layer)
    #[allow(dead_code)]
    hub_toml: Arc<HubTomlConfig>,
}

impl HubState {
    fn plugin_root(&self) -> std::result::Result<&PathBuf, crate::error::HubError> {
        self.plugin_root.as_ref().ok_or_else(|| {
            crate::error::HubError::new("plugin root not resolved — mine catalog unavailable")
        })
    }
}

// ---- serve entry -----------------------------------------------------------

pub async fn serve_http(
    mesh: Mesh,
    board_dir: PathBuf,
    port: u16,
    plugin_root: Option<PathBuf>,
) -> Result<()> {
    // Check if port is already bound
    if std::net::TcpListener::bind(format!("0.0.0.0:{port}")).is_err() {
        eprintln!("hub: port {port} already in use — daemon is already running");
        return Ok(());
    }

    let (board_tx, _) = broadcast::channel::<()>(64);
    let (mesh_tx, _) = broadcast::channel::<()>(64);

    let board = Board::open(board_dir, board::system_clock())?;

    let project_cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Startup restore: if a manifest exists and the plugin root is known,
    // restore missing items. Failures log to stderr but never abort startup.
    if project_cwd.join(".machine").join("install.toml").exists() {
        match &plugin_root {
            Some(root) => match mine::mine_restore(root, &project_cwd) {
                Ok(r) => eprintln!("hub mine: restore {r}"),
                Err(e) => eprintln!("hub mine: restore failed: {e}"),
            },
            None => eprintln!(
                "hub mine: install.toml present but plugin root unresolved — restore skipped"
            ),
        }
    }

    // Load hub.toml proxy config (load-only). Empty on missing/malformed file.
    let hub_toml = match &plugin_root {
        Some(root) => mine::load_hub_toml(root, &project_cwd),
        None => mine::load_hub_toml(&project_cwd, &project_cwd),
    };

    let state = HubState {
        mesh: Arc::new(mesh),
        board: Arc::new(board),
        registry: Arc::new(Registry::new()),
        sessions: Arc::new(RwLock::new(HashMap::new())),
        board_tx,
        mesh_tx,
        plugin_root,
        project_cwd,
        hub_toml: Arc::new(hub_toml),
    };

    let app = Router::new()
        .route("/", get(serve_ui))
        .route("/health", get(health))
        .route("/mcp", post(mcp_post))
        .route("/mcp", get(mcp_sse))
        .route("/ws", get(ws_handler))
        .route("/api/mine/list", get(api_mine_list))
        .route("/api/mine/install", post(api_mine_install))
        .route("/api/:verb", post(api_verb))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    eprintln!("hub: serving http://localhost:{port}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| crate::error::HubError::new(format!("bind failed: {e}")))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| crate::error::HubError::new(format!("serve error: {e}")))?;
    Ok(())
}

// ---- handlers --------------------------------------------------------------

/// The board UI, baked into the binary so release builds are self-contained.
const BOARD_UI: &str = include_str!("board_ui.html");

/// Source path of the UI, known at compile time. Used only when `HUB_DEV=1`
/// so a developer can edit `board_ui.html` and just refresh the browser —
/// no rebuild, no daemon restart.
const BOARD_UI_DEV_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/board_ui.html");

async fn serve_ui() -> Response {
    // In dev mode, re-read the HTML from disk on every request. Falls back to
    // the baked-in copy if the file is missing (e.g. binary moved).
    let html: std::borrow::Cow<'static, str> = if std::env::var_os("HUB_DEV").is_some() {
        match tokio::fs::read_to_string(BOARD_UI_DEV_PATH).await {
            Ok(s) => std::borrow::Cow::Owned(s),
            Err(_) => std::borrow::Cow::Borrowed(BOARD_UI),
        }
    } else {
        std::borrow::Cow::Borrowed(BOARD_UI)
    };
    (
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html.into_owned(),
    )
        .into_response()
}

async fn health() -> Json<Value> {
    Json(json!({ "hub": "machine-hub", "version": SERVER_VERSION }))
}

// POST /mcp — JSON-RPC request, synchronous response
async fn mcp_post(
    State(st): State<HubState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let session_id = headers
        .get("Mcp-Session-Id")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let req: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => {
            return Json(json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": { "code": -32700, "message": "parse error" }
            }))
            .into_response();
        }
    };

    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let method = req
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let params = req.get("params").cloned();

    let (result, new_session_id) = dispatch_http(&st, &method, params.as_ref(), session_id).await;

    let resp_body = match result {
        Ok(None) => json!({ "jsonrpc": "2.0", "id": id, "result": {} }),
        Ok(Some(r)) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
        Err(e) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32000, "message": e.to_string() }
        }),
    };

    let mut resp = Json(resp_body).into_response();
    if let Some(sid) = new_session_id {
        resp.headers_mut()
            .insert("Mcp-Session-Id", sid.parse().unwrap());
    }
    resp
}

// GET /mcp — SSE stream
async fn mcp_sse(State(st): State<HubState>, headers: HeaderMap) -> Response {
    use tokio_stream::wrappers::ReceiverStream;

    let sid = headers
        .get("Mcp-Session-Id")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let (tx, rx) = mpsc::channel::<String>(64);
    {
        let mut sessions = st.sessions.write().await;
        sessions.insert(sid.clone(), tx);
    }

    // Subscribe to registry changes and forward as SSE notifications
    let mut reg_rx = st.registry.subscribe();
    let sessions = Arc::clone(&st.sessions);
    let sid2 = sid.clone();
    tokio::spawn(async move {
        loop {
            if reg_rx.recv().await.is_err() {
                break;
            }
            while reg_rx.try_recv().is_ok() {}
            let notif = serde_json::to_string(&json!({
                "jsonrpc": "2.0",
                "method": "notifications/tools/list_changed"
            }))
            .unwrap();
            let sessions = sessions.read().await;
            if let Some(tx) = sessions.get(&sid2) {
                let _ = tx.send(notif).await;
            } else {
                break;
            }
        }
    });

    let stream = ReceiverStream::new(rx);
    use tokio_stream::StreamExt as _;
    let sse_stream =
        stream.map(|msg| Ok::<_, std::convert::Infallible>(format!("data:{}\n\n", msg)));

    let body = axum::body::Body::from_stream(sse_stream);
    (
        StatusCode::OK,
        [
            (axum::http::header::CONTENT_TYPE, "text/event-stream"),
            (axum::http::header::CACHE_CONTROL, "no-cache"),
        ],
        body,
    )
        .into_response()
}

// GET /ws — WebSocket handler
async fn ws_handler(ws: WebSocketUpgrade, State(st): State<HubState>) -> Response {
    ws.on_upgrade(|socket| handle_ws(socket, st))
}

async fn handle_ws(mut socket: WebSocket, st: HubState) {
    // Subscribe to mutation signals before sending initial state
    let mut board_rx = st.board_tx.subscribe();
    let mut mesh_rx = st.mesh_tx.subscribe();

    // Send full state on connect
    if let Some(snapshot) = build_snapshot(&st).await {
        let _ = socket.send(Message::Text(snapshot)).await;
    }

    loop {
        tokio::select! {
            // Board mutation
            _ = board_rx.recv() => {
                // drain
                while board_rx.try_recv().is_ok() {}
                if let Some(snap) = build_snapshot(&st).await {
                    if socket.send(Message::Text(snap)).await.is_err() { break; }
                }
            }
            // Mesh mutation
            _ = mesh_rx.recv() => {
                while mesh_rx.try_recv().is_ok() {}
                if let Some(snap) = build_snapshot(&st).await {
                    if socket.send(Message::Text(snap)).await.is_err() { break; }
                }
            }
            // Client message or close
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {} // ignore client messages on WS
                }
            }
        }
    }
}

async fn build_snapshot(st: &HubState) -> Option<String> {
    // Read board state
    let board_state = st.board.store.load();
    // Read mesh roster
    let mesh_state = st.mesh.current_state();

    let projects: Vec<Value> = board_state
        .projects
        .values()
        .map(|p| serde_json::to_value(p).unwrap())
        .collect();
    let columns: Vec<Value> = board_state
        .columns
        .values()
        .map(|c| serde_json::to_value(c).unwrap())
        .collect();
    let cards: Vec<Value> = board_state
        .cards
        .values()
        .map(|k| serde_json::to_value(k).unwrap())
        .collect();
    let comments: Vec<Value> = board_state
        .comments
        .values()
        .map(|c| serde_json::to_value(c).unwrap())
        .collect();
    let labels: Vec<Value> = board_state
        .labels
        .iter()
        .map(|(name, color)| json!({ "name": name, "color": color }))
        .collect();

    let snap = json!({
        "projects": projects,
        "columns": columns,
        "cards": cards,
        "comments": comments,
        "labels": labels,
        "roster": mesh_state.roster,
    });
    serde_json::to_string(&snap).ok()
}

// POST /api/<verb> — REST convenience for drag-and-drop
async fn api_verb(
    State(st): State<HubState>,
    axum::extract::Path(verb): axum::extract::Path<String>,
    body: axum::body::Bytes,
) -> Response {
    if !board::BOARD_VERBS.contains(&verb.as_str()) {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("unknown verb '{verb}'") })),
        )
            .into_response();
    }
    let args: Value = if body.is_empty() {
        json!({})
    } else {
        match serde_json::from_slice(&body) {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": e.to_string() })),
                )
                    .into_response()
            }
        }
    };
    match st.board.invoke(&verb, &args) {
        Ok(result) => {
            let _ = st.board_tx.send(());
            let result = if verb == "comment_add" {
                after_comment_add(&st, result).await
            } else {
                result
            };
            Json(result).into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Post-process a successful `comment_add` result. Both the REST and MCP call
/// sites route the result through here so the side effect and the response shape
/// can never drift apart: it fires the assignee notification, then strips the
/// internal-only `assignee` field so no client sees it.
async fn after_comment_add(st: &HubState, mut result: Value) -> Value {
    notify_assignee(st, &result);
    if let Some(obj) = result.as_object_mut() {
        obj.remove("assignee");
    }
    result
}

/// If `comment_result` contains a non-null `assignee`, post a mesh message to
/// that agent so it can receive the notification via `inbox`. Best-effort.
fn notify_assignee(st: &HubState, comment_result: &Value) {
    let assignee = match comment_result.get("assignee").and_then(|v| v.as_str()) {
        Some(a) if !a.is_empty() => a.to_string(),
        _ => return,
    };
    let comment = match comment_result.get("comment") {
        Some(c) => c.clone(),
        None => return,
    };
    let body = serde_json::to_string(&comment).unwrap_or_default();
    let _ = st.mesh.post(&json!({
        "agent_id": "hub",
        "to": assignee,
        "subject": "board:comment",
        "body": body,
    }));
    let _ = st.mesh_tx.send(());
}

// GET /api/mine/list — mine catalog with install status
async fn api_mine_list(State(st): State<HubState>) -> Response {
    match st.plugin_root() {
        Ok(root) => Json(mine::list_json(root, &st.project_cwd)).into_response(),
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// POST /api/mine/install — { "type": "skill"|"agent", "name": "..." }
async fn api_mine_install(State(st): State<HubState>, body: axum::body::Bytes) -> Response {
    let args: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };
    let Some(ty) = args
        .get("type")
        .and_then(|v| v.as_str())
        .and_then(MineType::parse)
    else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "type must be 'skill' or 'agent'" })),
        )
            .into_response();
    };
    let Some(name) = args.get("name").and_then(|v| v.as_str()) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "name is required" })),
        )
            .into_response();
    };
    let root = match st.plugin_root() {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };
    match mine::install_item(root, &st.project_cwd, ty, name) {
        Ok(result) => Json(result).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// ---- MCP dispatch (HTTP) ---------------------------------------------------

async fn dispatch_http(
    st: &HubState,
    method: &str,
    params: Option<&Value>,
    session_id: Option<String>,
) -> (
    std::result::Result<Option<Value>, crate::error::HubError>,
    Option<String>,
) {
    use crate::error::HubError;

    let mut new_session_id = None;

    let result: std::result::Result<Option<Value>, HubError> = match method {
        "initialize" => {
            let sid = session_id.unwrap_or_else(|| Uuid::new_v4().to_string());
            new_session_id = Some(sid.clone());
            // Ensure session exists
            let mut sessions = st.sessions.write().await;
            sessions.entry(sid).or_insert_with(|| mpsc::channel(64).0);
            Ok(Some(json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": { "listChanged": true } },
                "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION },
            })))
        }
        "notifications/initialized" | "initialized" => Ok(None),
        "ping" => Ok(Some(json!({}))),
        "tools/list" => Ok(Some(json!({ "tools": st.registry.list() }))),
        "tools/call" => {
            let params = match params {
                Some(p) => p,
                None => return (Err(HubError::new("missing params")), None),
            };
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));

            let result: std::result::Result<Value, HubError> = match name {
                "hub_register_tool" => {
                    let n = match args.get("name").and_then(|v| v.as_str()) {
                        Some(s) => s.to_string(),
                        None => return (Err(HubError::new("name is required")), None),
                    };
                    let d = match args.get("description").and_then(|v| v.as_str()) {
                        Some(s) => s.to_string(),
                        None => return (Err(HubError::new("description is required")), None),
                    };
                    let s = match args.get("input_schema").cloned() {
                        Some(v) => v,
                        None => return (Err(HubError::new("input_schema is required")), None),
                    };
                    let is_new = st.registry.register(n.clone(), d, s);
                    Ok(json!({ "registered": n, "is_new": is_new }))
                }
                "hub_unregister_tool" => {
                    let n = match args.get("name").and_then(|v| v.as_str()) {
                        Some(s) => s,
                        None => return (Err(HubError::new("name is required")), None),
                    };
                    let removed = st.registry.unregister(n);
                    Ok(json!({ "name": n, "removed": removed }))
                }
                verb if crate::registry::BUILTIN_VERBS.contains(&verb) => {
                    let mesh_result = match verb {
                        "register" => st.mesh.register(&args),
                        "roster" => st.mesh.roster(&args),
                        "claim" => st.mesh.claim(&args),
                        "release" => st.mesh.release(&args),
                        "claims" => st.mesh.claims(&args),
                        "post" => st.mesh.post(&args),
                        "inbox" => st.mesh.inbox(&args),
                        "read" => st.mesh.read(&args),
                        _ => Err(HubError::new(format!("unknown mesh verb '{verb}'"))),
                    };
                    // Fire mesh mutation signal for write verbs
                    if matches!(verb, "register" | "claim" | "release" | "post" | "read")
                        && mesh_result.is_ok()
                    {
                        let _ = st.mesh_tx.send(());
                    }
                    mesh_result
                }
                verb if crate::board::BOARD_VERBS.contains(&verb) => {
                    match st.board.invoke(verb, &args) {
                        Ok(r) => {
                            let _ = st.board_tx.send(());
                            Ok(if verb == "comment_add" {
                                after_comment_add(st, r).await
                            } else {
                                r
                            })
                        }
                        Err(e) => Err(e),
                    }
                }
                "hub_mine_list" => match st.plugin_root() {
                    Ok(root) => Ok(mine::list_json(root, &st.project_cwd)),
                    Err(e) => Err(e),
                },
                "hub_mine_install" => {
                    let ty = match args
                        .get("type")
                        .and_then(|v| v.as_str())
                        .and_then(MineType::parse)
                    {
                        Some(t) => t,
                        None => {
                            return (Err(HubError::new("type must be 'skill' or 'agent'")), None)
                        }
                    };
                    let n = match args.get("name").and_then(|v| v.as_str()) {
                        Some(s) => s.to_string(),
                        None => return (Err(HubError::new("name is required")), None),
                    };
                    match st.plugin_root() {
                        Ok(root) => mine::install_item(root, &st.project_cwd, ty, &n),
                        Err(e) => Err(e),
                    }
                }
                "hub_mine_restore" => match st.plugin_root() {
                    Ok(root) => mine::mine_restore(root, &st.project_cwd),
                    Err(e) => Err(e),
                },
                other => Err(HubError::new(format!("unknown verb '{other}'"))),
            };

            result.map(|r| {
                Some(json!({
                    "content": [{ "type": "text", "text": serde_json::to_string(&r).unwrap_or_default() }],
                    "structuredContent": r,
                }))
            })
        }
        other => Err(HubError::new(format!("unknown method '{other}'"))),
    };

    (result, new_session_id)
}
