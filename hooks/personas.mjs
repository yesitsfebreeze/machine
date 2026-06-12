#!/usr/bin/env node
// Stop hook: fires persona panel when feature completion is detected.
import { readFileSync } from "fs";

let raw = "";
try {
  raw = readFileSync(0, "utf8"); // fd 0 = stdin, works on all platforms
} catch {
  process.exit(0);
}

if (!raw.trim()) process.exit(0);

let data;
try {
  data = JSON.parse(raw);
} catch {
  process.exit(0);
}

if (data.stop_hook_active === true) process.exit(0);

const transcriptPath = data.transcript_path;
if (!transcriptPath) process.exit(0);

let lines;
try {
  lines = readFileSync(transcriptPath, "utf8");
} catch {
  process.exit(0);
}

const assistantTexts = [];
for (const line of lines.split("\n")) {
  const trimmed = line.trim();
  if (!trimmed) continue;
  try {
    const obj = JSON.parse(trimmed);
    if (obj.role !== "assistant") continue;
    let text = "";
    if (typeof obj.content === "string") {
      text = obj.content;
    } else if (Array.isArray(obj.content)) {
      for (const block of obj.content) {
        if (block.type === "text") text += block.text;
      }
    }
    if (text.length > 0) assistantTexts.push(text);
  } catch {
    continue;
  }
}

if (assistantTexts.length === 0) process.exit(0);

const lastMessage = assistantTexts[assistantTexts.length - 1];

if (lastMessage.length < 400) process.exit(0);

const patterns = [
  /implementation (is |now |)complete/i,
  /feature (is |now |)implemented/i,
  /feature (is |now |)complete/i,
  /successfully implemented/i,
  /has been implemented/i,
  /changes are (now |)complete/i,
  /is now working/i,
  /is now (fully |)functional/i,
  /all tests pass/i,
  /ready to (test|use|ship)/i,
  /the (fix|feature|system|component|renderer|shader|effect) (is |now |)(done|working|complete|ready)/i,
  /implementation done/i,
  /here.s what was (added|implemented|built|changed)/i,
];

const triggered = patterns.some((p) => p.test(lastMessage));
if (!triggered) process.exit(0);

process.stdout.write(
  "The feature above looks complete. Running the specialist panel review automatically - /personas\n"
);
process.exit(2);
