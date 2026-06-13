#!/usr/bin/env node
// Stop hook: workflow-friction sensor → propose a helper skill.
//
// Scans the session transcript for friction signals (errored tool results, retry
// flailing on the same file). When a session crosses the threshold, it forces the
// agent to pause and consider capturing the recurring task as a helper skill
// (.machine/skills/) so next time it goes smoothly. Nudges AT MOST ONCE per session.
//
// Pattern mirrors personas.mjs: read stdin JSON, read transcript, exit(2)+stdout
// to inject a continuation directive. Disable: remove from settings.json.
import { readFileSync, writeFileSync, existsSync } from "fs";
import { join } from "path";
import { tmpdir } from "os";

const FRICTION_THRESHOLD = 3; // distinct errored tool results in the session

function exit0() { process.exit(0); }

let raw = "";
try { raw = readFileSync(0, "utf8"); } catch { exit0(); }
if (!raw.trim()) exit0();

let data;
try { data = JSON.parse(raw); } catch { exit0(); }

// Avoid re-entrancy: if a prior Stop hook already forced a continuation, bail.
if (data.stop_hook_active === true) exit0();

const transcriptPath = data.transcript_path;
if (!transcriptPath) exit0();

// Once-per-session marker lives in the OS temp dir (not committed to the repo).
const sessionId = data.session_id || "unknown";
const markerPath = join(tmpdir(), `helper-suggest-${sessionId}.flag`);
if (existsSync(markerPath)) exit0();

let lines;
try { lines = readFileSync(transcriptPath, "utf8"); } catch { exit0(); }

let errorCount = 0;
const editCounts = new Map(); // file_path -> times edited (flailing signal)

for (const line of lines.split("\n")) {
  const trimmed = line.trim();
  if (!trimmed) continue;
  let obj;
  try { obj = JSON.parse(trimmed); } catch { continue; }

  const content = obj?.message?.content ?? obj?.content;
  if (!Array.isArray(content)) continue;

  for (const block of content) {
    if (!block || typeof block !== "object") continue;
    // Errored tool results come back as user-role tool_result blocks.
    if (block.type === "tool_result" && block.is_error === true) errorCount++;
    // Track repeated edits/writes to the same file.
    if (block.type === "tool_use" && (block.name === "Edit" || block.name === "Write")) {
      const fp = block.input?.file_path;
      if (fp) editCounts.set(fp, (editCounts.get(fp) || 0) + 1);
    }
  }
}

const maxEdits = editCounts.size ? Math.max(...editCounts.values()) : 0;
const flailing = maxEdits >= 4; // 4+ edits to one file in a session = churn
const frictionScore = errorCount + (flailing ? 2 : 0);

if (frictionScore < FRICTION_THRESHOLD) exit0();

// Mark this session so we nudge only once.
try { writeFileSync(markerPath, String(Date.now())); } catch { /* best effort */ }

const detail = [
  `Workflow friction this session: ${errorCount} errored tool result(s)` +
    (flailing ? `, plus heavy churn (one file edited ${maxEdits}x)` : "") + ".",
  "Before wrapping up, run the helper-skill reflection:",
  "1. Name the RECURRING task that caused the repeated failures (skip one-off typos / external outages).",
  "2. If this task will recur in THIS repo and a short doc would make it go right first time,",
  "   invoke the `helper` skill to capture it — it ASKS the user before writing anything.",
  "3. If it was genuinely a one-off, say so in one line and move on. Do not invent a helper for noise.",
].join("\n");

process.stdout.write(detail + "\n");
process.exit(2);
