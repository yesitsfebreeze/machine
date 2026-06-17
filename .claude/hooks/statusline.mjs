#!/usr/bin/env node
// Self-contained status line for the machine. No external binary, no project-state dep.
// Claude Code pipes a JSON status payload on stdin; we print one line to stdout.
// ANSI colors degrade gracefully if the terminal ignores them.
import { readFileSync } from "node:fs";
import { execSync } from "node:child_process";

const C = {
  reset: "\x1b[0m", dim: "\x1b[2m",
  cyan: "\x1b[36m", green: "\x1b[32m", yellow: "\x1b[33m",
  magenta: "\x1b[35m", blue: "\x1b[34m", red: "\x1b[31m",
  orange: "\x1b[38;5;208m",
};
const sep = `${C.dim} / ${C.reset}`;

// Context usage bar. Red above 50%, green below.
const contextBar = (pct) => {
  const width = 10;
  const filled = Math.max(0, Math.min(width, Math.round((pct / 100) * width)));
  const color = pct > 50 ? C.red : C.green;
  const fill = "█".repeat(filled);
  const empty = `${C.dim}${"░".repeat(width - filled)}${C.reset}`;
  return `${color}${fill}${empty} ${color}${pct}%${C.reset}`;
};

// Most-recent main-chain assistant usage gives current context occupancy.
const contextTokens = (transcriptPath) => {
  if (!transcriptPath) return 0;
  try {
    const lines = readFileSync(transcriptPath, "utf8").split("\n").filter(Boolean);
    for (let i = lines.length - 1; i >= 0; i--) {
      let obj;
      try { obj = JSON.parse(lines[i]); } catch { continue; }
      const u = obj?.message?.usage;
      if (u && obj?.message?.role === "assistant") {
        const t = (u.input_tokens || 0) + (u.cache_read_input_tokens || 0) + (u.cache_creation_input_tokens || 0);
        if (t > 0) return t;
      }
    }
  } catch {}
  return 0;
};

let d = {};
try { d = JSON.parse(readFileSync(0, "utf8") || "{}"); } catch {}

const cwd = d.workspace?.current_dir || d.cwd || process.cwd();
const projDir = d.workspace?.project_dir || cwd;

const git = (args) => {
  try {
    return execSync(`git ${args}`, { cwd, stdio: ["ignore", "pipe", "ignore"] })
      .toString().trim();
  } catch { return ""; }
};

const seg = [];

// 1 — Current working directory. Collapse $HOME to ~.
const home = process.env.HOME || process.env.USERPROFILE || "";
let cwdLabel = cwd;
if (home && cwd.startsWith(home)) cwdLabel = "~" + cwd.slice(home.length);
seg.push(`${C.blue}${cwdLabel}${C.reset}`);

// 2 — Git branch + local change count + ahead/behind + session line deltas.
const branch = git("rev-parse --abbrev-ref HEAD");
const add = d.cost?.total_lines_added || 0, rem = d.cost?.total_lines_removed || 0;
const deltas = (add || rem)
  ? `${C.green}+${add}${C.reset} ${C.red}-${rem}${C.reset}`
  : "";
if (branch) {
  const dirty = git("status --porcelain").split("\n").filter(Boolean).length;
  const ab = git("rev-list --left-right --count @{upstream}...HEAD").split(/\s+/);
  let g = `${C.magenta}⎇ ${branch}${C.reset}`;
  if (dirty) g += `${C.yellow}*${dirty}${C.reset}`;
  if (ab.length === 2) {
    const behind = +ab[0] || 0, ahead = +ab[1] || 0;
    if (ahead) g += `${C.green}↑${ahead}${C.reset}`;
    if (behind) g += `${C.red}↓${behind}${C.reset}`;
  }
  if (deltas) g += ` ${deltas}`;
  seg.push(g);
} else if (deltas) {
  seg.push(deltas);
}

// 4 — Model + context loading bar (fraction of the model's context window in use).
const dn = (d.model?.display_name || "").toLowerCase();
const limit = /1m|\[1m\]/.test(dn) ? 1_000_000 : 200_000;
const ctxTokens = contextTokens(d.transcript_path);
let m = d.model?.display_name ? `${C.orange}${d.model.display_name}${C.reset}` : "";
if (ctxTokens > 0) {
  const pct = Math.min(100, Math.round((ctxTokens / limit) * 100));
  m += (m ? " " : "") + contextBar(pct);
}
if (m) seg.push(m);

process.stdout.write(seg.join(sep));
