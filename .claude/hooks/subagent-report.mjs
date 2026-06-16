#!/usr/bin/env node
// SubagentStop hook: a subagent that joined the mesh must post a final report
// and release its claims before it stops. Agents that never registered on the
// mesh (read-only Explore/Plan/etc.) are left untouched.
import { loadStopHook } from "./stop-input.mjs";

const { transcript } = loadStopHook();

// Collect every tool_use name from the assistant turns.
const toolNames = [];
for (const line of transcript.split("\n")) {
  const trimmed = line.trim();
  if (!trimmed) continue;
  let obj;
  try {
    obj = JSON.parse(trimmed);
  } catch {
    continue;
  }
  if (obj.role !== "assistant" || !Array.isArray(obj.content)) continue;
  for (const block of obj.content) {
    if (block && block.type === "tool_use" && typeof block.name === "string") {
      toolNames.push(block.name);
    }
  }
}

const used = (suffix) => toolNames.some((n) => n.endsWith(suffix));

// Not a mesh participant - nothing to enforce.
if (!used("mesh__register")) process.exit(0);

// Joined the mesh: it must have posted (its report) and released its claims.
const posted = used("mesh__post");
const released = used("mesh__release");
if (posted && released) process.exit(0);

const missing = [];
if (!posted) missing.push("post a final report with `mcp__mesh__post` (goal, what you did, result, follow-ups)");
if (!released) missing.push("`mcp__mesh__release` every claim you hold");

process.stdout.write(JSON.stringify({
  decision: "block",
  reason: "Before you finish, close out on the mesh: " + missing.join(", and ") + ". Then stop.",
}));
process.exit(0);
