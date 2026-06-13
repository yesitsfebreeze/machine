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
};
const sep = `${C.dim} │ ${C.reset}`;

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

// Current working directory — first segment. Collapse $HOME to ~.
const home = process.env.HOME || process.env.USERPROFILE || "";
let cwdLabel = cwd;
if (home && cwd.startsWith(home)) cwdLabel = "~" + cwd.slice(home.length);
seg.push(`${C.dim}${cwdLabel}${C.reset}`);

// Project name — prefer /.proj/project.md "Name:", else dir basename.
let projName = projDir.replace(/[\\/]+$/, "").split(/[\\/]/).pop() || "·";
try {
  const pm = readFileSync(`${projDir}/.proj/project.md`, "utf8");
  const m = pm.match(/^\s*-?\s*\*\*Name:\*\*\s*(.+)$/m) || pm.match(/^#\s*Project facts\s*[—-]\s*(.+)$/m);
  if (m) projName = m[1].trim();
} catch {}
seg.push(`${C.cyan}${projName}${C.reset}`);

// Git branch + dirty count + ahead/behind.
const branch = git("rev-parse --abbrev-ref HEAD");
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
  seg.push(g);
}

// Model.
if (d.model?.display_name) seg.push(`${C.blue}${d.model.display_name}${C.reset}`);

// Output style (skip the default).
const style = d.output_style?.name;
if (style && style.toLowerCase() !== "default") seg.push(`${C.dim}${style}${C.reset}`);

// Cost + line deltas this session.
const cost = d.cost?.total_cost_usd;
if (typeof cost === "number" && cost > 0) seg.push(`${C.green}$${cost.toFixed(2)}${C.reset}`);
const add = d.cost?.total_lines_added || 0, rem = d.cost?.total_lines_removed || 0;
if (add || rem) seg.push(`${C.green}+${add}${C.reset}${C.dim}/${C.reset}${C.red}-${rem}${C.reset}`);

// 200k context warning.
if (d.exceeds_200k_tokens) seg.push(`${C.red}⚠ 200k+${C.reset}`);

process.stdout.write(seg.join(sep));
