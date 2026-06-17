//! hub — fleet inter-agent coordination daemon (kern's coordination sibling).
//!
//! Rust replacement for the prior `mesh/mesh.mjs`. kern owns *memory* (why
//! decisions were made); hub owns *live coordination* (who is here, who holds
//! what, who said what to whom) AND board state (kanban projects/columns/cards).
//!
//! Two transports:
//!   hub serve    HTTP+SSE singleton daemon on port 7777 (primary)
//!   hub mcp      MCP server over stdio (fallback / local test)

mod board;
mod board_http;
mod error;
mod machine;
mod mesh;
mod mine;
mod registry;
mod server;
mod state;
mod ulid_gen;

use error::Result;
use mesh::Mesh;
use std::path::PathBuf;
use std::process::Command;

const DATA_DIR: &str = ".mesh";
const BOARD_DIR: &str = ".board";
const DEFAULT_PORT: u16 = 7777;

const USAGE: &str = "hub — fleet inter-agent coordination daemon

Usage:
  hub serve [--port N]  Run the HTTP+SSE+WS singleton daemon (primary)
  hub mcp               Run the MCP server over stdio (fallback)
  hub machine [args..]  Ensure hub, launch an orchestrator Claude in an owned
                        PTY, and open the fzf switcher (args pass through to claude)
  hub gc                Reclaim TTL-expired messages and sweep dead claims
  hub --version         Show version

Data lives in repo-scoped, gitignored .mesh/ and .board/ directories.
Override with MESH_DIR / BOARD_DIR env vars.";

/// Resolve the mesh store dir. Repo-scoped, git-common-dir rooted.
fn data_dir() -> PathBuf {
    if let Ok(d) = std::env::var("MESH_DIR") {
        if !d.is_empty() {
            return PathBuf::from(d);
        }
    }
    if let Ok(output) = Command::new("git")
        .args(["rev-parse", "--git-common-dir"])
        .output()
    {
        if output.status.success() {
            let common = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !common.is_empty() {
                let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                let abs = if PathBuf::from(&common).is_absolute() {
                    PathBuf::from(&common)
                } else {
                    cwd.join(&common)
                };
                if let Some(parent) = abs.parent() {
                    return parent.join(DATA_DIR);
                }
            }
        }
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(DATA_DIR)
}

/// Resolve the board store dir (sibling of mesh dir).
fn board_data_dir() -> PathBuf {
    if let Ok(d) = std::env::var("BOARD_DIR") {
        if !d.is_empty() {
            return PathBuf::from(d);
        }
    }
    // Same parent as mesh dir but with .board suffix
    let mesh = data_dir();
    mesh.parent()
        .map(|p| p.join(BOARD_DIR))
        .unwrap_or_else(|| PathBuf::from(BOARD_DIR))
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("");
    if let Err(e) = run(cmd, &args[1.min(args.len())..]).await {
        eprintln!("hub: {e}");
        std::process::exit(1);
    }
}

async fn run(cmd: &str, rest: &[String]) -> Result<()> {
    match cmd {
        "serve" => {
            // Parse optional --port N
            let port = parse_port(rest).unwrap_or(DEFAULT_PORT);
            let mesh = Mesh::open(data_dir())?;
            let board_dir = board_data_dir();
            let plugin_root = mine::resolve_plugin_root(None);
            board_http::serve_http(mesh, board_dir, port, plugin_root).await
        }
        "mcp" => {
            let mesh = Mesh::open(data_dir())?;
            let board = board::Board::open(board_data_dir(), board::system_clock())?;
            let registry = registry::Registry::new();
            let plugin_root = mine::resolve_plugin_root(None);
            let project_cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            server::serve(mesh, board, registry, plugin_root, project_cwd).await
        }
        "machine" => {
            machine::run(rest)?;
            Ok(())
        }
        "gc" | "compact" => {
            let mesh = Mesh::open(data_dir())?;
            let n = mesh.gc()?;
            println!("hub: reclaimed {n} expired message(s); dead claims swept");
            Ok(())
        }
        "--version" | "-V" => {
            println!("hub {}", server::SERVER_VERSION);
            Ok(())
        }
        "" | "--help" | "-h" | "help" => {
            println!("{USAGE}");
            Ok(())
        }
        other => {
            eprintln!("hub: unknown command '{other}'\n\n{USAGE}");
            std::process::exit(1);
        }
    }
}

fn parse_port(args: &[String]) -> Option<u16> {
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        if a == "--port" {
            if let Some(p) = iter.next() {
                return p.parse().ok();
            }
        }
    }
    None
}
