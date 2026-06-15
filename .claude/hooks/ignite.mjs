#!/usr/bin/env node
// SessionStart hook: ignite machine mode.
// Detects per-repo state and hands the model a single instruction: invoke the
// `ignite` skill. The skill owns the playbook (caveman comms, oil nudge when
// unoiled, orchestration resume when oiled). This hook only gathers state and
// emits additionalContext. It never blocks or errors the session.
//
// Replaces caveman-startup.mjs + orchestrator-startup.mjs (unified entry point).
// Disable: remove this hook from hooks.json, or tell the agent "stop caveman".

import { readdirSync, readFileSync, existsSync } from "node:fs";
import { join } from "node:path";

// Statuses that mean "open work to resume" in the footer. `proposed`,
// `scheduled`, and `frozen` are v2 pre-fire taskboard states; the rest are
// in-flight or awaiting-approval states inherited from v1.
const ACTIVE = new Set([
  "proposed",
  "scheduled",
  "frozen",
  "pending-approval",
  "running",
  "changes-requested",
]);

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
  // fire_at drives v2 timer-resume (TB-009). It may be absent on older
  // v1 records; treat missing/unparseable as "no timer" so the hook never errors.
  const fireRaw = field(block, "fire_at").replace(/^["']|["']$/g, "");
  const fireMs = Date.parse(fireRaw);
  const fireAt = Number.isNaN(fireMs) ? null : fireMs;
  return { id, label: field(block, "label") || id, status, fireAt };
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
      : "Machine state: NOT OILED (/.machine absent) — ignite will nudge /oil; do not enter orchestration.",
  ];

  if (machineReady) {
    const open = collectSessions(join(root, ".machine", "sessions"));
    if (open.length > 0) {
      lines.push("Open taskboard from a prior session (resume the attention footer):");
      for (const a of open) lines.push(`[${a.id}] ${a.label} ${a.status.toUpperCase()}`);

      // TB-009 timer-resume: classify scheduled tasks by their persisted fire_at
      // vs now. Overdue (and not frozen) tasks are immediately eligible on resume;
      // the soonest still-future fire_at is where the wakeup re-schedules. This is
      // state-gathering only — the `ignite`/`orchestrate` skill owns the recompute
      // and the dependency/freeze eligibility check.
      const now = Date.now();
      const settling = open.filter((a) => a.status === "scheduled" && a.fireAt !== null);
      const overdue = settling.filter((a) => a.fireAt <= now).map((a) => a.id);
      const future = settling.filter((a) => a.fireAt > now);
      if (overdue.length > 0) {
        lines.push(
          `Overdue (fire_at elapsed while the session was down): ${overdue.join(", ")}. ` +
            "Recompute eligibility (dependencies approved, not frozen) and launch the due ones now.",
        );
      }
      if (future.length > 0) {
        const soonest = future.reduce((m, a) => (a.fireAt < m.fireAt ? a : m));
        lines.push(
          `Soonest future fire_at: [${soonest.id}] at ${new Date(soonest.fireAt).toISOString()} — schedule the single wakeup to it.`,
        );
      }
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
