#!/usr/bin/env node
// SessionStart hook: resume orchestrator mode when subagents are still open.
// Scans /.machine/sessions/*.md (excluding README.md) and, for any file whose
// frontmatter status is pending-approval, running, or changes-requested, emits a
// short digest via additionalContext so the attention footer resumes after a
// session restart. Emits nothing (and exits 0) when nothing qualifies, the
// directory is absent, or a file is malformed. Never errors or blocks the session.

import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

const ACTIVE = new Set(["pending-approval", "running", "changes-requested"]);

function field(frontmatter, key) {
  const match = frontmatter.match(new RegExp(`^${key}:\\s*(.+?)\\s*$`, "m"));
  return match ? match[1].trim() : "";
}

function parse(text) {
  // Frontmatter is the first block between leading --- fences.
  const match = text.match(/^---\r?\n([\s\S]*?)\r?\n---/);
  if (!match) return null;
  const block = match[1];
  const status = field(block, "status");
  if (!ACTIVE.has(status)) return null;
  const id = field(block, "id");
  const label = field(block, "label");
  if (!id) return null;
  return { id, label: label || id, status };
}

function collect(dir) {
  const out = [];
  for (const name of readdirSync(dir)) {
    if (name === "README.md" || !name.endsWith(".md")) continue;
    try {
      const entry = parse(readFileSync(join(dir, name), "utf8"));
      if (entry) out.push(entry);
    } catch {
      // Malformed or unreadable file: skip silently.
    }
  }
  return out;
}

try {
  const root = process.env.CLAUDE_PROJECT_DIR || process.cwd();
  const dir = join(root, ".machine", "sessions");
  const agents = collect(dir);
  if (agents.length === 0) process.exit(0);

  const lines = agents.map(
    (a) => `[${a.id}] ${a.label} ${a.status.toUpperCase()}`,
  );
  const context = [
    "Orchestrator mode has open subagents from a prior session.",
    "Invoke the `orchestrate` skill, resume the attention footer, and rebuild it from /.machine/sessions/:",
    ...lines,
  ].join("\n");

  process.stdout.write(
    JSON.stringify({
      hookSpecificOutput: {
        hookEventName: "SessionStart",
        additionalContext: context,
      },
    }),
  );
} catch {
  // Any unexpected failure degrades to silence.
  process.exit(0);
}
