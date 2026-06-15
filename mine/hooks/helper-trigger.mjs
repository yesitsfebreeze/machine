#!/usr/bin/env node
// UserPromptSubmit hook: deterministic helper-skill trigger ("tag cloud").
//
// Reads the project helper registry (.machine/skills/registry.json) and matches the
// user's prompt against each helper's `tags`. Any match force-injects a reminder
// (additionalContext) telling the agent to READ the helper doc before acting.
// This is the RELIABLE trigger path — it does not rely on model judgment the way
// Skill-tool description matching does.
//
// Disable: remove this hook from settings.json. Empty registry = silent no-op.
import { readFileSync } from "fs";
import { join } from "path";

function exit0() { process.exit(0); }

let raw = "";
try { raw = readFileSync(0, "utf8"); } catch { exit0(); }
if (!raw.trim()) exit0();

let data;
try { data = JSON.parse(raw); } catch { exit0(); }

const prompt = typeof data.prompt === "string" ? data.prompt : "";
if (!prompt.trim()) exit0();

const projectDir = process.env.CLAUDE_PROJECT_DIR || process.cwd();
const registryPath = join(projectDir, ".machine", "skills", "registry.json");

let registry;
try {
  registry = JSON.parse(readFileSync(registryPath, "utf8"));
} catch {
  exit0(); // no registry yet — nothing to trigger
}

const helpers = Array.isArray(registry?.helpers) ? registry.helpers : [];
if (helpers.length === 0) exit0();

// Normalize prompt: lowercase, collapse non-word chars to spaces, pad with
// spaces so single-word tags can be matched on word boundaries.
const hay = " " + prompt.toLowerCase().replace(/[^a-z0-9]+/g, " ").trim() + " ";

// Normalize the tag the same way as the prompt, then boundary-match. Because
// `hay` is space-collapsed and space-padded, one check handles single tokens AND
// multi-word phrases identically, anchored on both ends — no special-casing, no
// loose substring surprises. (Tags match whole words; use a stem if you want
// "migrate" to also catch "migrating".)
function tagMatches(tag) {
  const t = String(tag).toLowerCase().replace(/[^a-z0-9]+/g, " ").trim();
  if (!t) return false;
  return hay.includes(" " + t + " ");
}

const hits = [];
for (const h of helpers) {
  const tags = Array.isArray(h?.tags) ? h.tags : [];
  const matched = tags.filter(tagMatches);
  if (matched.length > 0) {
    hits.push({ ...h, score: matched.length, matched });
  }
}

if (hits.length === 0) exit0();

hits.sort((a, b) => b.score - a.score);

const lines = hits.slice(0, 4).map((h) => {
  const file = join(".machine", "skills", h.file || `${h.name}.md`);
  const why = h.summary ? ` — ${h.summary}` : "";
  return `- ${h.name} (${file})${why} [matched: ${h.matched.join(", ")}]`;
});

const context = [
  "Project helper skill(s) match this prompt. These capture how recurring tasks",
  "are done correctly in THIS repo. READ the relevant helper doc BEFORE acting —",
  "do not improvise a method the helper already documents:",
  ...lines,
].join("\n");

process.stdout.write(
  JSON.stringify({
    hookSpecificOutput: {
      hookEventName: "UserPromptSubmit",
      additionalContext: context,
    },
  }),
);
process.exit(0);
