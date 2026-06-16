//! hub — fleet inter-agent coordination daemon (kern's coordination sibling).
//!
//! Rust replacement for the prior `mesh/mesh.mjs`. kern owns *memory* (why
//! decisions were made); hub owns *live coordination* (who is here, who holds
//! what, who said what to whom). Three categories of dynamic state — roster,
//! claims, messages — exposed as eight MCP verbs over stdio.
//!
//! State is a single JSON file under a repo-scoped, gitignored `.mesh/`
//! directory (the dir name is unchanged for data continuity), shared by every
//! git worktree of the repo. Cross-process atomicity comes from an OS-atomic
//! lock directory plus atomic rename on write.

mod error;
mod mesh;
mod registry;
mod server;
mod state;
mod ulid_gen;

use error::Result;
use mesh::Mesh;
use std::path::PathBuf;
use std::process::Command;

const DATA_DIR: &str = ".mesh";

const USAGE: &str = "hub — fleet inter-agent coordination daemon (kern's coordination sibling)

Usage:
  hub mcp        Run the MCP server over stdio (the fleet attaches here)
  hub gc         Reclaim TTL-expired messages and sweep dead claims
  hub --version  Show version

Data lives in a repo-scoped, gitignored .mesh/ directory at the repo root, shared
by every git worktree of the repo (a single JSON state file). Override with MESH_DIR.";

/// Resolve the hub store. Repo-scoped, not cwd-scoped: every git worktree of the
/// same repository shares one store. Resolution order:
///   1. MESH_DIR env override.
///   2. The repo root that owns this cwd — the parent of git's common dir.
///   3. Fall back to cwd when not inside a git repository.
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
                // `common` is `.git` (relative) in a main worktree, or an
                // absolute path in a linked worktree. Its parent is the shared
                // repo root in both cases.
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

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("");
    if let Err(e) = run(cmd).await {
        eprintln!("hub: {e}");
        std::process::exit(1);
    }
}

async fn run(cmd: &str) -> Result<()> {
    match cmd {
        "mcp" => {
            let mesh = Mesh::open(data_dir())?;
            let registry = registry::Registry::new();
            server::serve(mesh, registry).await
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
