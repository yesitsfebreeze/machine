#!/usr/bin/env node
// SessionStart hook: ignite machine mode.
// Detects per-repo state and hands the model a single instruction: invoke the
// `ignite` skill. The skill owns the playbook (caveman comms, oil-me nudge when
// unoiled, orchestration resume when oiled). This hook only gathers state and
// emits additionalContext. It never blocks or errors the session.
//
// Replaces caveman-startup.mjs + orchestrator-startup.mjs (unified entry point).
// Disable: remove this hook from hooks.json, or tell the agent "stop caveman".

import { readdirSync, readFileSync, existsSync } from "node:fs";
import { join } from "node:path";

const ACTIVE = new Set(["pending-approval", "running", "changes-requested"]);

function field(frontmatter, key) {
  const match = frontmatter.match(new RegExp(`^${key}:\\s*(.+?)\\s*$`, "m"));
  return match ? match[1].trim() : "";
}

function parseSession(text) {
  const match = text.match(/^---\r?\n([\s\S]*?)\r?\n---/);
  if (!match) return null;
  const block = match[1];
  const status = field(block, "status");
  if (!ACTIVE.has(status)) return null;
  const id = field(block, "id");
  if (!id) return null;
  return { id, label: field(block, "label") || id, status };
}

function collectSessions(dir) {
  const out = [];
  try {
    for (const name of readdirSync(dir)) {
      if (name === "README.md" || !name.endsWith(".md")) continue;
      try {
        const entry = parseSession(readFileSync(join(dir, name), "utf8"));
        if (entry) out.push(entry);
      } catch {
        // Malformed or unreadable session file: skip silently.
      }
    }
  } catch {
    // Sessions dir absent or unreadable: no open work.
  }
  return out;
}

try {
  const root = process.env.CLAUDE_PROJECT_DIR || process.cwd();
  const machineReady = existsSync(join(root, ".machine"));

  const lines = [
    "Caveman comm mode is ON by default this session.",
    "Invoke the `ignite` skill now to bring up machine mode and follow it.",
    machineReady
      ? "Machine state: OILED (/.machine present)."
      : "Machine state: NOT OILED (/.machine absent) — ignite will nudge /oil-me; do not enter orchestration.",
  ];

  if (machineReady) {
    const open = collectSessions(join(root, ".machine", "sessions"));
    if (open.length > 0) {
      lines.push("Open subagents from a prior session (resume the attention footer):");
      for (const a of open) lines.push(`[${a.id}] ${a.label} ${a.status.toUpperCase()}`);
    }
  }

  process.stdout.write(
    JSON.stringify({
      hookSpecificOutput: {
        hookEventName: "SessionStart",
        additionalContext: lines.join("\n"),
      },
    }),
  );
} catch {
  // Any unexpected failure degrades to silence.
  process.exit(0);
}
