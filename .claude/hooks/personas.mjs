#!/usr/bin/env node
// Stop hook: fires persona panel when feature completion is detected.
import { loadStopHook } from "./stop-input.mjs";

const { transcript } = loadStopHook();

const assistantTexts = [];
for (const line of transcript.split("\n")) {
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

// Short replies are conversational turns, not completed work - skip the
// completion-pattern scan below. 400 chars ~ the floor of a real wrap-up summary.
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

// Stop hooks block-and-continue via JSON on stdout with exit 0. Exit 2 makes
// Claude Code read STDERR (ignoring stdout) - an empty-stderr exit 2 surfaces as
// a "No stderr output" hook error and the directive is lost.
process.stdout.write(JSON.stringify({
  decision: "block",
  reason: "The feature above looks complete. Running the specialist panel review automatically - invoke the `personas` skill.",
}));
process.exit(0);
