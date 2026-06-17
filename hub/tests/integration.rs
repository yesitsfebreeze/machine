//! Integration test: spawn the built `hub` binary, drive a minimal MCP session
//! over stdio, and assert on the responses. Marked `#[ignore]` because it needs
//! the release binary to exist (`cargo build --release`).

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

#[test]
#[ignore = "requires `cargo build --release` to have produced target/release/hub"]
fn mcp_initialize_and_register_roundtrip() {
    let bin = env!("CARGO_BIN_EXE_hub");
    let tmp = std::env::temp_dir().join(format!("hub-it-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);

    let mut child = Command::new(bin)
        .arg("mcp")
        .env("MESH_DIR", tmp.join(".mesh"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn hub mcp");

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    let send = |stdin: &mut std::process::ChildStdin, msg: &str| {
        stdin.write_all(msg.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();
        stdin.flush().unwrap();
    };
    let recv = |stdout: &mut BufReader<std::process::ChildStdout>| -> serde_json::Value {
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        serde_json::from_str(line.trim()).unwrap()
    };

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#,
    );
    let init = recv(&mut stdout);
    assert_eq!(init["result"]["serverInfo"]["name"], "hub");

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"register","arguments":{"agent_id":"a","branch":"gitfs/a","prompt_ptr":"p"}}}"#,
    );
    let reg = recv(&mut stdout);
    assert_eq!(reg["result"]["structuredContent"]["epoch"], 1);

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"roster","arguments":{"agent_id":"a"}}}"#,
    );
    let roster = recv(&mut stdout);
    assert_eq!(
        roster["result"]["structuredContent"]["agents"][0]["liveness"],
        "alive"
    );

    drop(stdin); // EOF -> child exits
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&tmp);
}
