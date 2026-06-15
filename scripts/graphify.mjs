#!/usr/bin/env node
// graphify — build a fast, navigable capability graph of THIS repo (the machine).
//
// Walks the whole machine: registered agents/skills/hooks/rules/output-styles,
// the unregistered `mine/` kit, and the `.machine` project layer. Emits a flat,
// greppable artifact at `.machine/graph.json` that the default agent reads so
// nothing in the kit stays undiscovered. Zero runtime deps (node + fs only).
//
// Repo-local by design: invoked from the untracked `.git/hooks/pre-push` hook
// (installed by `scripts/bootstrap.sh`) or manually via `just graphify`. It is
// NOT a registered machine hook, so it never travels to other repos.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const r = (...p) => path.join(ROOT, ...p);
const rel = (p) => path.relative(ROOT, p);

const exists = (p) => fs.existsSync(p);
const read = (p) => fs.readFileSync(p, "utf8");
const lsDirs = (p) =>
  exists(p) ? fs.readdirSync(p, { withFileTypes: true }).filter((d) => d.isDirectory()).map((d) => d.name) : [];
const lsFiles = (p, ext) =>
  exists(p)
    ? fs.readdirSync(p, { withFileTypes: true }).filter((d) => d.isFile() && (!ext || d.name.endsWith(ext))).map((d) => d.name)
    : [];

// Minimal YAML-frontmatter scrape: leading `name:`/`description:`/`paths:`.
// description may be a folded `>` block. Good enough for an index.
function frontmatter(file) {
  if (!exists(file)) return {};
  const txt = read(file);
  const m = txt.match(/^---\r?\n([\s\S]*?)\r?\n---/);
  const out = {};
  if (!m) return out;
  const fm = m[1];
  const name = fm.match(/^name:\s*(.+)$/m);
  if (name) out.name = name[1].trim().replace(/^["']|["']$/g, "");
  const dline = fm.match(/^description:\s*(.*)$/m);
  if (dline) {
    if (/^[>|]/.test(dline[1].trim()) || dline[1].trim() === "") {
      const after = fm.slice(fm.indexOf(dline[0]) + dline[0].length);
      const cont = [];
      for (const line of after.split(/\r?\n/)) {
        if (/^\s+\S/.test(line)) cont.push(line.trim());
        else if (line.trim() === "") continue;
        else break;
      }
      out.description = cont.join(" ").trim();
    } else {
      out.description = dline[1].trim().replace(/^["']|["']$/g, "");
    }
  }
  const paths = fm.match(/^paths:\s*(.+)$/m);
  if (paths) out.paths = paths[1].trim().replace(/^["']|["']$/g, "");
  return out;
}

const registered = (() => {
  try { return JSON.parse(read(r(".claude-plugin/plugin.json"))); } catch { return {}; }
})();
const regAgents = new Set((registered.agents || []).map((p) => path.basename(p, ".md")));
const regSkills = new Set((registered.skills || []).map((p) => path.basename(p)));

const nodes = [];
const edges = [];
const add = (n) => { nodes.push(n); return n.id; };
const link = (from, to, kind) => edges.push({ from, to, kind });

// --- agents -----------------------------------------------------------------
for (const [dir, isKit] of [[".claude/agents", false], ["mine/agents", true]]) {
  for (const f of lsFiles(r(dir), ".md")) {
    const fm = frontmatter(r(dir, f));
    const base = path.basename(f, ".md");
    add({ id: `agent:${base}`, type: "agent", name: fm.name || base, path: rel(r(dir, f)),
      kit: isKit, registered: isKit ? false : regAgents.has(base), description: fm.description || "" });
  }
}

// --- skills -----------------------------------------------------------------
for (const [dir, isKit] of [[".claude/skills", false], ["mine/skills", true]]) {
  for (const d of lsDirs(r(dir))) {
    const skillFile = ["SKILL.md", "skill.md"].map((n) => r(dir, d, n)).find(exists);
    const fm = skillFile ? frontmatter(skillFile) : {};
    add({ id: `skill:${d}`, type: "skill", name: fm.name || d, path: rel(r(dir, d)),
      kit: isKit, registered: isKit ? false : regSkills.has(d), description: fm.description || "" });
  }
}

// --- hooks (registered events + scripts, plus kit hook scripts) -------------
try {
  const hj = JSON.parse(read(r(".claude/hooks/hooks.json")));
  for (const [event, groups] of Object.entries(hj.hooks || {})) {
    const evId = add({ id: `event:${event}`, type: "hook-event", name: event, registered: true });
    for (const g of groups) {
      for (const h of g.hooks || []) {
        const cmd = h.command || "";
        const sm = cmd.match(/([\w./-]+\.(?:mjs|js|sh|py))/);
        const script = sm ? path.basename(sm[1]) : cmd;
        const sId = add({ id: `hook:${script}`, type: "hook-script", name: script, registered: true });
        link(evId, sId, "fires");
      }
    }
  }
} catch {}
for (const f of lsFiles(r("mine/hooks"))) {
  add({ id: `hook:${f}`, type: "hook-script", name: f, path: rel(r("mine/hooks", f)), kit: true, registered: false });
}

// --- rules ------------------------------------------------------------------
for (const f of lsFiles(r(".claude/rules"), ".md")) {
  const fm = frontmatter(r(".claude/rules", f));
  add({ id: `rule:${f}`, type: "rule", name: path.basename(f, ".md"), path: rel(r(".claude/rules", f)), registered: true, paths: fm.paths || "*" });
}

// --- output styles ----------------------------------------------------------
for (const f of lsFiles(r(".claude/output-styles"), ".md")) {
  add({ id: `style:${path.basename(f, ".md")}`, type: "output-style", name: path.basename(f, ".md"), path: rel(r(".claude/output-styles", f)), registered: true });
}

// --- project layer (.machine) ----------------------------------------------
for (const f of lsFiles(r(".machine"), ".md")) {
  add({ id: `machine:${f}`, type: "project-doc", name: path.basename(f, ".md"), path: rel(r(".machine", f)), registered: true });
}
for (const sub of ["personas", "plans", "specs", "skills"]) {
  for (const f of lsFiles(r(".machine", sub), ".md")) {
    add({ id: `machine:${sub}/${f}`, type: `project-${sub}`, name: path.basename(f, ".md"), path: rel(r(".machine", sub, f)), registered: true });
  }
}

// --- cross-reference edges: doc body mentions another node's name -----------
const named = nodes.filter((n) => n.name && (n.type === "agent" || n.type === "skill"));
function bodyOf(node) {
  const p = node.path && r(node.path);
  if (!p || !exists(p)) return "";
  if (fs.statSync(p).isDirectory()) {
    const f = ["SKILL.md", "skill.md"].map((n) => path.join(p, n)).find(exists);
    return f ? read(f) : "";
  }
  return read(p);
}
for (const src of nodes) {
  if (src.type !== "agent" && src.type !== "skill") continue;
  const body = bodyOf(src);
  if (!body) continue;
  for (const tgt of named) {
    if (tgt.id === src.id) continue;
    const re = new RegExp(`\\b${tgt.name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\b`);
    if (re.test(body)) link(src.id, tgt.id, "references");
  }
}

// --- stats + emit -----------------------------------------------------------
const byType = {};
for (const n of nodes) byType[n.type] = (byType[n.type] || 0) + 1;
const orphans = nodes.filter((n) => n.kit && !n.registered).map((n) => n.id);

const graph = {
  schema: 1,
  source: "scripts/graphify.mjs",
  generated_at: new Date().toISOString(),
  stats: { nodes: nodes.length, edges: edges.length, byType, orphanKitItems: orphans.length },
  orphans,
  nodes,
  edges,
};

const outDir = r(".machine");
if (!exists(outDir)) fs.mkdirSync(outDir, { recursive: true });
const out = r(".machine/graph.json");
fs.writeFileSync(out, JSON.stringify(graph, null, 2) + "\n");
process.stderr.write(`graphify: ${nodes.length} nodes, ${edges.length} edges, ${orphans.length} orphan kit items -> ${rel(out)}\n`);
