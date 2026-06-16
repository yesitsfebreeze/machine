#!/usr/bin/env node
// mine-sync.mjs — declarative mine-kit sync.
//
// Source of truth: <projectDir>/.machine/mine.json
//
// Format:
//   {
//     "agents": ["<name>", ...],     // desired mine-kit agents
//     "skills": ["<name>", ...],     // desired mine-kit skills
//     "hooks":  [],                  // desired mine-kit hooks (future)
//     "_rejected": { "<name>": "<reason>" },  // optional, for /mine memory
//     "_synced": {                   // written by THIS script — do not edit
//       "agents": [...], "skills": [...], "hooks": []
//     }
//   }
//
// The script diffs `agents`/`skills` against `_synced.*` (what it last put in)
// to determine exactly what to add and what to remove. It never touches
// anything not tracked in `_synced`, so base bundled agents/skills are safe.
//
// Called by:
//   - /mine skill (Phase 5) after the user confirms a new manifest
//   - /oil after re-index (applies existing manifest idempotently)
//   - Directly: node mine-sync.mjs [--project-dir <path>]
//
// Exits 1 only when PLUGIN_ROOT or mine kit cannot be resolved.
// All other problems are logged and non-fatal.

import {
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
  readdirSync,
  cpSync,
} from 'node:fs';
import { join, dirname, resolve, basename } from 'node:path';
import { fileURLToPath } from 'node:url';

// ---------------------------------------------------------------------------
// Resolve roots
// ---------------------------------------------------------------------------

const PLUGIN_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');

const argIdx = process.argv.indexOf('--project-dir');
const PROJECT_DIR =
  argIdx !== -1
    ? resolve(process.argv[argIdx + 1])
    : process.env.CLAUDE_PROJECT_DIR || process.cwd();

// ---------------------------------------------------------------------------
// Load manifest
// ---------------------------------------------------------------------------

const manifestPath = join(PROJECT_DIR, '.machine', 'mine.json');

if (!existsSync(manifestPath)) {
  // No manifest — nothing to sync.
  process.exit(0);
}

let manifest;
try {
  manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
} catch (e) {
  process.stderr.write(`mine-sync: cannot parse mine.json: ${e.message}\n`);
  process.exit(0);
}

const wantAgents = new Set(Array.isArray(manifest.agents) ? manifest.agents : []);
const wantSkills = new Set(Array.isArray(manifest.skills) ? manifest.skills : []);

// _synced tracks what this script last installed — diff target, not the
// base bundled set, so base agents/skills are never touched.
const synced = manifest._synced || { agents: [], skills: [], hooks: [] };
const syncedAgents = new Set(Array.isArray(synced.agents) ? synced.agents : []);
const syncedSkills = new Set(Array.isArray(synced.skills) ? synced.skills : []);

// ---------------------------------------------------------------------------
// Resolve mine kit
// ---------------------------------------------------------------------------

let mineRoot = null;

try {
  const envMd = readFileSync(join(PROJECT_DIR, '.machine', 'ENV.md'), 'utf8');
  const m = envMd.match(/^export MACHINE_MINE=(.+)$/m);
  if (m) {
    const candidate = JSON.parse(m[1].trim());
    if (existsSync(candidate)) mineRoot = candidate;
  }
} catch {}

if (!mineRoot) {
  const fallback = join(PLUGIN_ROOT, 'mine');
  if (existsSync(fallback)) mineRoot = fallback;
}

if (!mineRoot) {
  process.stderr.write('mine-sync: mine kit not found (ENV.md missing or stale). Run /oil first.\n');
  process.exit(1);
}

// ---------------------------------------------------------------------------
// Load plugin.json
// ---------------------------------------------------------------------------

const pluginJsonPath = join(PLUGIN_ROOT, '.claude-plugin', 'plugin.json');
const pluginJson = JSON.parse(readFileSync(pluginJsonPath, 'utf8'));

// ---------------------------------------------------------------------------
// Compute diff (vs _synced, not vs kit membership)
// ---------------------------------------------------------------------------

const addAgents    = [...wantAgents].filter((n) => !syncedAgents.has(n));
const removeAgents = [...syncedAgents].filter((n) => !wantAgents.has(n));
const addSkills    = [...wantSkills].filter((n) => !syncedSkills.has(n));
const removeSkills = [...syncedSkills].filter((n) => !wantSkills.has(n));

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function sanitize(content) {
  return content.replace(/^permissionMode:\s*bypassPermissions\s*\n/m, '');
}

const slottedAgents = [], removedAgents = [], slottedSkills = [], removedSkills = [];

for (const name of addAgents) {
  const src = join(mineRoot, 'agents', `${name}.md`);
  if (!existsSync(src)) {
    process.stderr.write(`mine-sync: agent "${name}" not in mine kit — skipped\n`);
    continue;
  }
  const dst = join(PLUGIN_ROOT, '.claude', 'agents', `${name}.md`);
  writeFileSync(dst, sanitize(readFileSync(src, 'utf8')));
  slottedAgents.push(name);
}

for (const name of removeAgents) {
  const dst = join(PLUGIN_ROOT, '.claude', 'agents', `${name}.md`);
  if (existsSync(dst)) { rmSync(dst); removedAgents.push(name); }
}

for (const name of addSkills) {
  const src = join(mineRoot, 'skills', name);
  if (!existsSync(src)) {
    process.stderr.write(`mine-sync: skill "${name}" not in mine kit — skipped\n`);
    continue;
  }
  const dst = join(PLUGIN_ROOT, '.claude', 'skills', name);
  mkdirSync(dst, { recursive: true });
  cpSync(src, dst, { recursive: true, force: true });
  slottedSkills.push(name);
}

for (const name of removeSkills) {
  const dst = join(PLUGIN_ROOT, '.claude', 'skills', name);
  if (existsSync(dst)) {
    rmSync(dst, { recursive: true, force: true });
    removedSkills.push(name);
  }
}

// ---------------------------------------------------------------------------
// Rewrite plugin.json
// ---------------------------------------------------------------------------

// Remove previously-synced entries, then add currently-wanted entries.
// Base bundled entries are untouched because they were never in syncedAgents/Skills.
const removedAgentPaths = new Set([...removedAgents].map((n) => `./.claude/agents/${n}.md`));
const removedSkillPaths = new Set([...removedSkills].map((n) => `./.claude/skills/${n}`));

pluginJson.agents = [
  ...(pluginJson.agents || []).filter((p) => !removedAgentPaths.has(p)),
];
pluginJson.skills = [
  ...(pluginJson.skills || []).filter((p) => !removedSkillPaths.has(p)),
];

// Add newly slotted (idempotent — skip if already in the list).
const existingAgentPaths = new Set(pluginJson.agents);
for (const name of slottedAgents) {
  const p = `./.claude/agents/${name}.md`;
  if (!existingAgentPaths.has(p)) pluginJson.agents.push(p);
}
const existingSkillPaths = new Set(pluginJson.skills);
for (const name of slottedSkills) {
  const p = `./.claude/skills/${name}`;
  if (!existingSkillPaths.has(p)) pluginJson.skills.push(p);
}

writeFileSync(pluginJsonPath, JSON.stringify(pluginJson, null, 2) + '\n');

// Mirror the installed cache atomically (best-effort).
const cachePath = join(
  process.env.HOME || '',
  '.claude', 'plugins', 'cache', 'machine', 'machine',
  pluginJson.version || '0.0.0', '.claude-plugin', 'plugin.json',
);
if (existsSync(cachePath)) {
  try { writeFileSync(cachePath, JSON.stringify(pluginJson, null, 2) + '\n'); } catch {}
}

// ---------------------------------------------------------------------------
// Update _synced in mine.json
// ---------------------------------------------------------------------------

manifest._synced = {
  agents: [...wantAgents],
  skills: [...wantSkills],
  hooks: Array.isArray(manifest.hooks) ? manifest.hooks : [],
};
writeFileSync(manifestPath, JSON.stringify(manifest, null, 2) + '\n');

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------

const lines = ['mine-sync:'];
if (slottedAgents.length)  lines.push(`  slotted agents:  ${slottedAgents.join(', ')}`);
if (removedAgents.length)  lines.push(`  removed agents:  ${removedAgents.join(', ')}`);
if (slottedSkills.length)  lines.push(`  slotted skills:  ${slottedSkills.join(', ')}`);
if (removedSkills.length)  lines.push(`  removed skills:  ${removedSkills.join(', ')}`);
if (
  !slottedAgents.length && !removedAgents.length &&
  !slottedSkills.length && !removedSkills.length
) {
  lines.push('  already in sync — nothing to do');
}

process.stdout.write(lines.join('\n') + '\n');
