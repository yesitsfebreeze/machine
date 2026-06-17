//! machine — single-binary orchestrator: hub (start-or-attach) + a Claude-scoped
//! PTY multiplexer with a fast, modal switcher.
//!
//!   hub machine          ensure hub on :7777, launch an orchestrator Claude in an
//!                        owned PTY, and drop into the switcher.
//!
//! Switcher keys (overview): an always-on input line (between the help line and
//! the list) fuzzy-filters as you type; the first match is auto-selected.
//!   type         fuzzy filter  ↑/↓ or Ctrl-j/k  move
//!   Enter        attach        Ctrl-n           new agent
//!   Esc / Ctrl-c quit
//! While attached: double-tap the detach key (default `Ctrl-O Ctrl-O`) to return
//! to the overview — a single press passes through to Claude. If the Claude
//! instance exits on its own, you also land back on the overview. Override the
//! key with MACHINE_DETACH (e.g. ctrl-g).
//!
//! Only PTY-owned agents are live-attachable; native-listed sessions from other
//! terminals are shown for awareness.

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute, queue,
    style::{Attribute, Print, SetAttribute},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// Default detach key: Ctrl-O. Ctrl+letter is layout-independent (works on any
// keyboard), unlike Ctrl-\ which can't be typed on many non-US layouts. Override
// with MACHINE_DETACH, e.g. MACHINE_DETACH=ctrl-g or MACHINE_DETACH=ctrl-]
const DEFAULT_DETACH: char = 'o';
const SCROLLBACK_CAP: usize = 256 * 1024;
const PORT: u16 = 7777;

/// One PTY-backed Claude session that `machine` owns.
struct Agent {
    name: String,
    session_id: String,
    writer: Mutex<Box<dyn Write + Send>>,
    master: Mutex<Box<dyn portable_pty::MasterPty + Send>>,
    scrollback: Arc<Mutex<Vec<u8>>>,
    focused: Arc<AtomicBool>,
    alive: Arc<AtomicBool>,
}

impl Agent {
    /// Spawn a `claude` session in a fresh PTY. The reader thread always drains
    /// output into the scrollback ring; it also writes to stdout while focused.
    fn spawn(name: &str, cwd: &std::path::Path, extra_args: &[String]) -> std::io::Result<Arc<Agent>> {
        let (cols, rows) = terminal::size().unwrap_or((120, 32));
        let pair = native_pty_system()
            .openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(to_io)?;

        let session_id = uuid::Uuid::new_v4().to_string();
        let mut cmd = CommandBuilder::new("claude");
        cmd.arg("--name");
        cmd.arg(name);
        cmd.arg("--session-id");
        cmd.arg(&session_id);
        for a in extra_args {
            cmd.arg(a);
        }
        cmd.cwd(cwd);

        let mut child = pair.slave.spawn_command(cmd).map_err(to_io)?;
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader().map_err(to_io)?;
        let writer = pair.master.take_writer().map_err(to_io)?;

        let scrollback = Arc::new(Mutex::new(Vec::with_capacity(8192)));
        let focused = Arc::new(AtomicBool::new(false));
        let alive = Arc::new(AtomicBool::new(true));

        let agent = Arc::new(Agent {
            name: name.to_string(),
            session_id,
            writer: Mutex::new(writer),
            master: Mutex::new(pair.master),
            scrollback: scrollback.clone(),
            focused: focused.clone(),
            alive: alive.clone(),
        });

        // output pump
        {
            let scrollback = scrollback.clone();
            let focused = focused.clone();
            let alive = alive.clone();
            thread::spawn(move || {
                let mut buf = [0u8; 8192];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            let chunk = &buf[..n];
                            if focused.load(Ordering::Relaxed) {
                                let mut out = std::io::stdout();
                                let _ = out.write_all(chunk);
                                let _ = out.flush();
                            }
                            let mut sb = scrollback.lock().unwrap();
                            sb.extend_from_slice(chunk);
                            if sb.len() > SCROLLBACK_CAP {
                                let cut = sb.len() - SCROLLBACK_CAP;
                                sb.drain(0..cut);
                            }
                        }
                    }
                }
                alive.store(false, Ordering::Relaxed);
            });
        }

        // reaper: flip alive when the child exits
        {
            let alive = alive.clone();
            thread::spawn(move || {
                let _ = child.wait();
                alive.store(false, Ordering::Relaxed);
            });
        }

        Ok(agent)
    }

    fn resize(&self, cols: u16, rows: u16) {
        if let Ok(m) = self.master.lock() {
            let _ = m.resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 });
        }
    }

    fn write_input(&self, bytes: &[u8]) {
        if let Ok(mut w) = self.writer.lock() {
            let _ = w.write_all(bytes);
            let _ = w.flush();
        }
    }
}

fn to_io<E: std::fmt::Display>(e: E) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
}

/// Restores terminal state (raw mode, alt screen) on drop — so an error or panic
/// mid-switcher/attach never leaves the user's terminal wedged.
struct TermGuard {
    alt: bool,
}
impl TermGuard {
    fn raw() -> std::io::Result<Self> {
        terminal::enable_raw_mode()?;
        Ok(TermGuard { alt: false })
    }
    fn raw_alt() -> std::io::Result<Self> {
        execute!(std::io::stdout(), EnterAlternateScreen)?;
        terminal::enable_raw_mode()?;
        Ok(TermGuard { alt: true })
    }
}
impl Drop for TermGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        if self.alt {
            let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        }
    }
}

/// Case-insensitive subsequence ("fuzzy") match: every char of `q` appears in
/// `label` in order. Empty query matches everything.
fn fuzzy(q: &str, label: &str) -> bool {
    if q.is_empty() {
        return true;
    }
    let mut qi = q.chars().map(|c| c.to_ascii_lowercase());
    let mut want = qi.next();
    for lc in label.chars().map(|c| c.to_ascii_lowercase()) {
        if Some(lc) == want {
            want = qi.next();
            if want.is_none() {
                return true;
            }
        }
    }
    false
}

/// A native session surfaced by `claude agents --json` (for awareness only).
#[derive(serde::Deserialize)]
struct NativeAgent {
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
    kind: Option<String>,
    name: Option<String>,
    status: Option<String>,
    state: Option<String>,
}

fn native_agents(cwd: &std::path::Path) -> Vec<NativeAgent> {
    let out = std::process::Command::new("claude")
        .args(["agents", "--json", "--cwd"])
        .arg(cwd)
        .output();
    match out {
        Ok(o) if o.status.success() => serde_json::from_slice(&o.stdout).unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// TCP-probe the hub port; spawn a detached `hub serve` if nothing answers.
fn ensure_hub() {
    if std::net::TcpStream::connect(("127.0.0.1", PORT)).is_ok() {
        return;
    }
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe)
            .arg("serve")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
        for _ in 0..50 {
            if std::net::TcpStream::connect(("127.0.0.1", PORT)).is_ok() {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}

/// Background cache of native sessions. `claude agents` is a heavy subprocess, so
/// it is refreshed off-thread and the switcher only ever reads the snapshot —
/// never spawns a process in its render/input loop.
struct NativeCache {
    rows: Arc<Mutex<Vec<(String, String)>>>, // (label, session_id)
}
impl NativeCache {
    fn spawn(cwd: std::path::PathBuf) -> Self {
        let rows = Arc::new(Mutex::new(Vec::new()));
        let out = rows.clone();
        thread::spawn(move || loop {
            let v: Vec<(String, String)> = native_agents(&cwd)
                .into_iter()
                .map(|n| {
                    let sid = n.session_id.unwrap_or_default();
                    let label = format!(
                        "· {}  [{}]  {} ({})",
                        n.name.unwrap_or_else(|| "-".into()),
                        sid.chars().take(8).collect::<String>(),
                        n.kind.unwrap_or_default(),
                        n.status.or(n.state).unwrap_or_default()
                    );
                    (label, sid)
                })
                .collect();
            *out.lock().unwrap() = v;
            thread::sleep(Duration::from_secs(4));
        });
        NativeCache { rows }
    }
    fn snapshot(&self) -> Vec<(String, String)> {
        self.rows.lock().unwrap().clone()
    }
}

enum Picked {
    Attach(usize),
    NewAgent,
    Quit,
}

/// fzf-style picker with an always-on input line between the help line and the
/// list. Typing filters by fuzzy match; the first match is auto-selected and
/// Enter confirms it. Reads cached data only; redraws solely on change. Runs in
/// raw mode + alt screen, restored by TermGuard on any exit path.
///
/// Keys: type to filter · ↑/↓ or Ctrl-j/k move · Enter attach · Ctrl-n new ·
/// Esc/Ctrl-c quit. Plain letters feed the filter, so navigation and actions
/// live on arrows and Ctrl-chords.
fn switcher(agents: &[Arc<Agent>], native: &NativeCache) -> std::io::Result<Picked> {
    let mut query = String::new();
    let mut sel: usize = 0;
    let mut stdout = std::io::stdout();
    let _guard = TermGuard::raw_alt()?;
    let mut last_sig = String::new();

    // Fixed layout rows: title (0), help (1), input (2), list (4+).
    const INPUT_ROW: u16 = 2;
    const LIST_ROW: u16 = 4;

    let result = loop {
        // Rows: owned agents (attachable) first, then native-only (awareness).
        let mut rows: Vec<(String, bool)> = Vec::new();
        for a in agents {
            let live = a.alive.load(Ordering::Relaxed);
            rows.push((
                format!(
                    "{} {}  [{}]  owned",
                    if live { "●" } else { "✗" },
                    a.name,
                    &a.session_id[..8]
                ),
                true,
            ));
        }
        let owned_ids: Vec<&str> = agents.iter().map(|a| a.session_id.as_str()).collect();
        for (label, sid) in native.snapshot() {
            if owned_ids.iter().any(|o| sid.starts_with(*o) || o.starts_with(&sid)) {
                continue;
            }
            rows.push((label, false));
        }

        let filtered: Vec<usize> = rows
            .iter()
            .enumerate()
            .filter(|(_, (label, _))| fuzzy(&query, label))
            .map(|(i, _)| i)
            .collect();
        if sel >= filtered.len() {
            sel = filtered.len().saturating_sub(1);
        }

        // Only redraw when something actually changed (no idle flicker/lag).
        let sig = format!(
            "{}|{}|{}",
            query,
            sel,
            filtered.iter().map(|&i| rows[i].0.as_str()).collect::<Vec<_>>().join("\n")
        );
        if sig != last_sig {
            queue!(stdout, cursor::Hide, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
            queue!(
                stdout,
                SetAttribute(Attribute::Bold),
                Print("machine — overview"),
                SetAttribute(Attribute::Reset),
                cursor::MoveTo(0, 1),
                Print(format!(
                    "  type to filter · ↑/↓ move · enter attach · ctrl-n new · esc quit · {0} {0} returns here",
                    detach_key().1
                )),
                cursor::MoveTo(0, INPUT_ROW),
                Print(format!("  > {}", query)),
                cursor::MoveTo(0, LIST_ROW),
            )?;
            if filtered.is_empty() {
                queue!(stdout, Print("  (no matches)"))?;
            }
            for (vis, &ri) in filtered.iter().enumerate() {
                let (label, _) = &rows[ri];
                queue!(stdout, cursor::MoveTo(0, LIST_ROW + vis as u16))?;
                if vis == sel {
                    queue!(
                        stdout,
                        SetAttribute(Attribute::Reverse),
                        Print(format!("> {}", label)),
                        SetAttribute(Attribute::Reset),
                    )?;
                } else {
                    queue!(stdout, Print(format!("  {}", label)))?;
                }
            }
            // Park the real cursor at the end of the input line ("  > " = 4 cols).
            let col = 4 + query.chars().count() as u16;
            queue!(stdout, cursor::MoveTo(col, INPUT_ROW), cursor::Show)?;
            stdout.flush()?;
            last_sig = sig;
        }

        // Poll briefly so native-cache updates surface without blocking input.
        if !event::poll(Duration::from_millis(150))? {
            continue;
        }
        let Event::Key(k) = event::read()? else { continue };
        let down = |s: &mut usize| {
            if *s + 1 < filtered.len() {
                *s += 1;
            }
        };
        // Single mode: plain letters feed the filter; navigation/actions live on
        // arrows and Ctrl-chords. Any filter edit resets to the first match.
        match (k.code, k.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => break Picked::Quit,
            (KeyCode::Char('n'), KeyModifiers::CONTROL) => break Picked::NewAgent,
            (KeyCode::Esc, _) => break Picked::Quit,
            (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::CONTROL) => down(&mut sel),
            (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                sel = sel.saturating_sub(1)
            }
            (KeyCode::Enter, _) => {
                if let Some(&ri) = filtered.get(sel) {
                    if ri < agents.len() {
                        break Picked::Attach(ri);
                    }
                }
            }
            (KeyCode::Backspace, _) => {
                query.pop();
                sel = 0;
            }
            (KeyCode::Char(c), m) if m == KeyModifiers::NONE || m == KeyModifiers::SHIFT => {
                query.push(c);
                sel = 0;
            }
            _ => {}
        }
    };

    Ok(result)
}

/// Resolve the detach key(s) into the raw control byte(s) to scan for, plus a
/// human label. The key is detached by pressing it TWICE in a row (tmux-style
/// prefix): a single press passes through to Claude, so the key is never lost.
fn detach_key() -> (u8, String) {
    let spec = std::env::var("MACHINE_DETACH").unwrap_or_default();
    // accept "ctrl-x" / "c-x" / "^x" / "x"; map to Ctrl byte (upper & 0x1f)
    let ch = spec
        .to_lowercase()
        .trim_start_matches("ctrl-")
        .trim_start_matches("c-")
        .trim_start_matches('^')
        .chars()
        .next()
        .unwrap_or(DEFAULT_DETACH);
    let byte = (ch.to_ascii_uppercase() as u8) & 0x1f;
    (byte, format!("Ctrl-{}", ch.to_ascii_uppercase()))
}

/// Attach the real terminal to an agent's PTY: raw stdin → pty, pty → stdout
/// (via the agent's focused flag). The detach key returns to the overview;
/// agent exit also returns.
fn attach(agent: &Arc<Agent>) -> std::io::Result<()> {
    let mut stdout = std::io::stdout();
    let _guard = TermGuard::raw()?;
    execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;

    // replay recent output so the screen has context
    {
        let sb = agent.scrollback.lock().unwrap();
        let _ = stdout.write_all(&sb);
        let _ = stdout.flush();
    }
    agent.focused.store(true, Ordering::Relaxed);

    // SIGWINCH → resize flag
    let winch = Arc::new(AtomicBool::new(false));
    let _ = signal_hook::flag::register(signal_hook::consts::SIGWINCH, winch.clone());

    let (dkey, _label) = detach_key();
    // Double-tap to detach: a lone prefix is held briefly, then flushed to Claude
    // if no second prefix follows. Two within the window → overview.
    const DOUBLE_TAP_MS: u64 = 400;
    let mut pending: Option<Instant> = None;
    let stdin_fd = libc::STDIN_FILENO;
    let mut buf = [0u8; 4096];
    let mut detach_now = false;
    loop {
        if !agent.alive.load(Ordering::Relaxed) {
            break; // instance exited on its own → back to overview
        }
        if winch.swap(false, Ordering::Relaxed) {
            if let Ok((cols, rows)) = terminal::size() {
                agent.resize(cols, rows);
            }
        }
        // a held prefix that wasn't followed by a second press → send it through
        if let Some(t) = pending {
            if t.elapsed() >= Duration::from_millis(DOUBLE_TAP_MS) {
                agent.write_input(&[dkey]);
                pending = None;
            }
        }
        // poll stdin with a short timeout so exit/winch/flush are caught promptly
        let mut pfd = libc::pollfd { fd: stdin_fd, events: libc::POLLIN, revents: 0 };
        let r = unsafe { libc::poll(&mut pfd, 1, 100) };
        if r > 0 && (pfd.revents & libc::POLLIN) != 0 {
            let n = unsafe { libc::read(stdin_fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
            if n <= 0 {
                break;
            }
            for &b in &buf[..n as usize] {
                if pending.is_some() {
                    pending = None;
                    if b == dkey {
                        detach_now = true; // second prefix → overview
                        break;
                    }
                    agent.write_input(&[dkey]); // lone prefix, then this byte
                    agent.write_input(&[b]);
                } else if b == dkey {
                    pending = Some(Instant::now()); // hold; await a possible twin
                } else {
                    agent.write_input(&[b]);
                }
            }
            if detach_now {
                break;
            }
        }
    }

    agent.focused.store(false, Ordering::Relaxed);
    drop(_guard);
    execute!(stdout, Print("\r\n"))?;
    Ok(())
}

/// Entry point for `hub machine`.
pub fn run(rest: &[String]) -> std::io::Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    ensure_hub();
    let native = NativeCache::spawn(cwd.clone());

    // One orchestrator Claude runs in the back. Extra args pass through to claude.
    let orchestrator = Agent::spawn("orchestrator", &cwd, rest)?;
    let mut agents: Vec<Arc<Agent>> = vec![orchestrator];
    let counter = AtomicUsize::new(1);

    loop {
        match switcher(&agents, &native)? {
            Picked::Quit => break,
            Picked::Attach(i) => {
                if let Some(a) = agents.get(i).cloned() {
                    attach(&a)?;
                }
            }
            Picked::NewAgent => {
                let n = counter.fetch_add(1, Ordering::Relaxed);
                let name = format!("agent-{n}");
                match Agent::spawn(&name, &cwd, &[]) {
                    Ok(a) => {
                        let idx = agents.len();
                        agents.push(a);
                        let a = agents[idx].clone();
                        attach(&a)?;
                    }
                    Err(e) => eprintln!("machine: spawn failed: {e}"),
                }
            }
        }
    }
    Ok(())
}
