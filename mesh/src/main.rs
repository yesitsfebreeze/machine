//! mesh CLI. Mirrors kern's shape: the MCP server is a subcommand launched over
//! stdio (`mesh mcp`), with `gc`/`compact` maintenance subcommands.

use std::process::ExitCode;

use mesh::Daemon;

const USAGE: &str = "\
mesh — fleet inter-agent communication daemon (kern's coordination sibling)

Usage:
  mesh mcp        Run the MCP server over stdio (the fleet attaches here)
  mesh gc         Reclaim TTL-expired messages and sweep dead claims
  mesh compact    Alias for gc (kept for parity with kern's maintenance verbs)
  mesh --help     Show this help
  mesh --version  Show version

Data lives in a per-cwd, gitignored .mesh/ directory (LMDB primary + SQLite-WAL
journal), mirroring kern's .kern/.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("");

    match cmd {
        "mcp" => run_mcp(),
        "gc" | "compact" => run_gc(),
        "--version" | "-V" => {
            println!("mesh {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        "" | "--help" | "-h" | "help" => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("mesh: unknown command '{other}'\n\n{USAGE}");
            ExitCode::FAILURE
        }
    }
}

/// Open the daemon for the current working directory.
fn open() -> Result<Daemon, Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    Ok(Daemon::open_in(&cwd)?)
}

fn run_mcp() -> ExitCode {
    let daemon = match open() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("mesh: failed to open store: {e}");
            return ExitCode::FAILURE;
        }
    };
    match mesh::mcp::serve_stdio(&daemon) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("mesh: mcp server error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run_gc() -> ExitCode {
    let daemon = match open() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("mesh: failed to open store: {e}");
            return ExitCode::FAILURE;
        }
    };
    match daemon.gc() {
        Ok(n) => {
            println!("mesh: reclaimed {n} expired message(s); dead claims swept");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("mesh: gc error: {e}");
            ExitCode::FAILURE
        }
    }
}
