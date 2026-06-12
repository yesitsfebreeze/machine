# Src Cleanup Loop

## Goal

Drive `src/` toward **zero duplication, minimum file count, maximum density**. Every line earns its place. Every file owns one responsibility. Shared logic lives in one place, referenced from many.

The agent self-paces: searches, finds gaps, acts, restarts. The user never has to point at a target — `git ls-files src/` plus an audit pass at every iter surfaces the next move.

## Loop pattern (every iter)

```
SEARCH → FIND → ACT → RESTART
```

1. **SEARCH.** Vicky-first (always). `vicky:query` the cleanup topic + open the matching `glimmer-cleanup-iter-*` conclusion to confirm prior cleanup invariants (helper-extraction patterns, merge-decision matrix, file-private gating discipline). Then run the audit script (LOC per file, top-level decl counts, cross-file dup hits, dead-code candidates). The vault is the first source of truth; the audit is second.
2. **FIND.** Pick ONE highest-leverage gap from the audit. Either (a) a dedupe block ≥ 6 lines in ≥ 2 files, (b) a sibling-file pair that should merge by responsibility, (c) a dead symbol with zero refs, or (d) a > 1000 LOC file with > 1 responsibility (split candidate).
3. **ACT.** Apply ONE atomic change. `odin check src` green between every extract / merge / delete. Commit unit = one logical cleanup, not one file (a math helper used by 4 files is one commit even though it touches 5 files).
4. **RESTART.** Re-audit. Schedule next iter. Name the next concrete target in the wake reason.

Skip a step → the iter is invalid.

## Hard rules

- **One commit per iter.** Atomic deliverable: one logical cleanup OR one vicky write (when blocked). Never two commits per cycle.
- **No bundling user WIP.** Stage by explicit file list (`git add <paths>`), never `git add src/`. Before commit, run `git diff --cached` and confirm every staged hunk matches the cycle's intended set. Anything outside → `git restore --staged <path>`. Per `cleanup-loop-no-bundle-user-WIP-rule-post-e293280`.
- **`odin check src` green between every extract.** Never batch extracts without a check between.
- **`odin test src` green before commit.** No exceptions.
- **Self-pacing only.** Never ask the user "what should I clean next". The audit has the answer.
- **No tracker artifacts.** No `cleanup.json` / `refactor.md` / `progress.md` / `status.md`. Track in-session via `TodoWrite` only. Per `CLAUDE.md` desired-state rule.
- **Stop on user word.** "stop loop" / "exit loop" / "pause" → enqueue summary + exit.

## Three forces (resolve in order)

1. **Dedupe wins.** Two near-identical blocks → extract shared. Always.
2. **Consolidate wins.** Two small files with overlapping responsibility → merge.
3. **Split only when forced.** A file crosses 1000 LOC AND mixes responsibilities → split by responsibility, not line count.

A 1100-LOC file doing exactly one thing well stays. Line count is a smell, not a verdict.

## Pass decision gate (run every iter, before picking a target)

After Vicky-first + audit, before any edit:

1. **Dup check.** Cross-file dup block ≥ 6 lines in ≥ 3 callers → **Pass 1 dedupe**.
2. **Dead-code check.** Symbol with zero refs across `src/` + `src/shaders/` → **Pass 4 dead-code** (cheapest win, runtime bit-identical).
3. **Consolidate check.** Two files < 200 LOC sharing a responsibility, no lifetime / import-set conflict → **Pass 2 consolidate**.
4. **Split check.** A file > 1000 LOC AND > 1 responsibility (audit reveals mixed concerns) → **Pass 3 split**.
5. **Perf check.** Hot-path file flagged in audit AND a focused candidate identified → **Pass 5 perf** (isolated commit, bench gate).
6. **Default → enqueue + skip.** Nothing safely shippable → `vicky:enqueue` findings + next-candidate list, exit clean.

Never run two passes in one iter.

## Pass 0 — Audit

Per `*.odin` under `src/`:

- **LOC** (raw line count).
- **Responsibilities** (1-sentence each — if > 1 sentence needed, file is mixed).
- **Duplication hits** — blocks of ≥ 6 lines that appear in ≥ 2 files, or ≥ 3 near-identical structs / procs.
- **Dead code** — unreferenced procs / types / constants. Verify via Grep across `src/` + `src/shaders/` + tests before deleting.
- **Reusable candidates** — math / geometry / layout / GPU-helper code currently inlined that other files would benefit from.
- **Hot-path candidates** — code on the per-frame critical path with obvious inefficiency (allocation in inner loop, redundant work, missing `#soa`, etc.). Confirm via `just bench` before / after — never optimize on intuition.
- **Quality score** 1–5: 5 = single responsibility, no duplication, hot path tuned; 1 = mixed, duplicated, untouched.

Rank work queue by: (duplication count desc) → (LOC desc) → (quality score asc).

## Pass 1 — Dedupe (highest priority)

For each duplication hit:

- **Math / linear algebra** → consolidate into `src/tetra.odin` (Vec3 + foundational geometry). Grep first.
- **AABB / bounds / spatial ops** → `src/aabb.odin` if exists, else `src/tetra.odin`.
- **GPU descriptor builders (RenderPipeline / bind group / pipeline layout / depth-stencil)** → `src/render_pipeline.odin`. Existing helpers: `blend_ftb_premultiplied`, `color_target_blended`, `fragment_state_default`, `depth_stencil_revz`, `bind_group_layout`, `pipeline_layout`, `bind_group`.
- **Tooling-side wrappers (tracer / bench / screenshot helpers)** → `src/tooling/` package. Glimmer → tooling one-way only.
- **Tet / face-adjacency / cake ops** → existing tet module. Never duplicate into a new file.
- **WGSL-shared structs** → single Odin source of truth with `#assert size_of` + `#assert offset_of` gates. WGSL side reads the same layout.

Extraction rules:

1. Move the block, replace call sites, delete originals.
2. If the extracted proc takes > 4 params, the abstraction is wrong — rethink the boundary.
3. `odin check src` after each extract.
4. If the proc has only 1 caller after extract, inline it back. Premature abstraction is duplication's quieter sibling.

## Pass 2 — Consolidate

Merge candidates:

- **Two files < 200 LOC sharing a responsibility** → merge.
- **A file that only exists to hold one type + one constructor** → fold into its consumer.
- **Parallel `foo.odin` + `foo_gpu.odin` split where the GPU side is < 150 LOC** → merge unless the GPU half pulls heavy wgpu imports the CPU half does not need.
- **Scattered test helpers across `*_test.odin`** → single `test_util.odin`.

Do not merge:

- Files with different lifetimes (frame-transient vs long-lived state).
- Files with sharply different import sets (pure-math file vs wgpu-heavy file).
- Files crossing the render / physics / bake boundary.
- Files crossing the glimmer / tooling package boundary.

Comment-strip on merged files: keep `#assert` contracts + non-obvious invariants (cursor-warp delta drop, FRAME_TOTAL share-fallback, outward-normal contract, reverse-Z + FTB blend rationale). Drop section dividers, restates-the-code narratives, future-tense remarks.

## Pass 3 — Split (only when forced)

Split a file when **both** hold:

- **> 1000 LOC**, AND
- **> 1 responsibility** (audit revealed mixed concerns).

Split axis is **responsibility**, never line count. A 1200-LOC file doing one job stays. A 600-LOC file doing three jobs splits.

Naming: derived child files share the parent prefix (`bake.odin` → `bake_weld.odin`, `bake_cdt3d.odin`). Avoid generic suffixes (`_util`, `_helpers`) — they become dumping grounds.

## Pass 4 — Dead code

For each unreferenced symbol:

1. Grep across `src/`, `src/tooling/`, `src/shaders/`.
2. Check if it is `@(private="file")` — file-private dead code is always safe to delete.
3. Check git log on the symbol — if introduced < 7 days ago, verify it is not mid-feature (diagnostic scaffold the user is actively iterating on).
4. Delete. Do not move dead code to a "maybe useful" file. Git remembers.

## Pass 5 — Performance

Only for files flagged hot-path in audit.

1. Capture baseline: `just bench-baseline`.
2. Apply one focused change.
3. `just bench` twice (two-batch convention per `CLAUDE.md`).
4. Decision rule per `CLAUDE.md` bench discipline. Revert if regression on owned section.

Do not bundle perf changes with dedupe / consolidate / split passes. Perf needs isolated measurement.

## Per-iter workflow

```
1. Vicky-first: query + DQL + read matching conclusion.
2. Audit: LOC + dup hits + dead symbols + hot-path candidates.
3. Phase gate: pick ONE pass (1 / 2 / 3 / 4 / 5) or skip.
4. Read target file(s) end-to-end.
5. Apply ONE atomic change. `odin check src` between every extract.
6. `odin test src` → green.
7. Pre-commit review: `git diff --cached` matches intended file set; user WIP unstaged.
8. Commit per cleanup unit. Caveman commit style.
9. Push origin master.
10. Re-audit. Schedule next iter (`ScheduleWakeup`, delaySeconds: 1800 default — cleanup runs on user-driven commits, no harness event to gate on).
```

## Postflight (every iter)

P1. **Audit delta.**
   - File count vs prior iter.
   - Total LOC vs prior iter.
   - Remaining dup-hit count.
   - Remaining dead-symbol count.
   - Trend MUST be flat or down on every axis the iter targeted.

P2. **Schedule next iter** via `ScheduleWakeup`. `delaySeconds: 1800` (cleanup runs in cache-miss window — no event signal to wait for). Reason names the concrete next target from the just-completed audit.

P3. **End-state hard rule.** Either ONE new commit on master OR clean exit via `vicky:enqueue` "skipped: <reason>". Never both. Never neither. Zero-write iter = red flag (Vicky-first was skipped, audit was misread, or WIP gate failed and was not enqueued).

## WIP gate (mandatory at iter start)

If `git status` shows uncommitted code changes the agent did NOT make:

1. Identify scope: file paths + diff size.
2. If WIP touches files outside the cycle's intended set → cycle proceeds; stage by explicit file list and skip user-WIP files.
3. If WIP touches the SAME file the cycle wants to edit (e.g. main.odin mid-prefix-sweep while user adds a loop block) → split via `git add -p` or revert + reapply just the cycle hunks. Never `git add src/`.
4. If working tree is mid-debug session (diagnostic scaffolds + bug-fix WIP overlapping the cleanup target) → **skip this iter**. `vicky:enqueue` "skipped: WIP gate failed, <fingerprint>" + reschedule. Cleanup on top of debug WIP creates merge friction the next commit cannot untangle cleanly.

## Done criteria (per file)

A file is done when:

- One responsibility, expressed in one sentence.
- Zero duplicated blocks ≥ 6 lines with any other file.
- Zero unreferenced symbols.
- ≤ 1000 LOC, OR > 1000 LOC with single responsibility (justified in audit notes).
- Hot paths confirmed via `just bench` (no regression on owned section).
- `odin test src` green. `odin check src` clean. WGSL `#assert` gates intact.

## Termination

Loop self-terminates when:

- Audit returns zero gaps across all five passes — no dup hits ≥ 6 lines, no dead symbols, no consolidate candidates, no > 1000-LOC mixed-responsibility files, no flagged hot paths with focused candidates.
- OR three consecutive iters produce zero commits (every remaining gap requires effort > one-iter budget).

On termination: enqueue final summary (iter count, files reduced, LOC reduced, dup hits resolved, dead symbols deleted, files merged, perf wins, remaining backlog).

Other stop conditions:

- User says "stop loop" / "exit loop" / "pause".
- `odin check src` red and not recoverable in the same iter.
- `odin test src` red — never commit on red. Revert to clean tree, enqueue diagnosis.

## Invariants — do not break

- Hard tabs everywhere.
- Reverse-Z, +Z up / +Y forward / +X right right-handed world frame — never refactor away.
- SoA verts, AoS tets — never interleave under "consolidation".
- Tet-as-atom — never duplicate per-tet state into a parallel grid / probe / lightmap.
- WGSL ↔ Odin GPU struct layout `#assert` gates — re-run after every touch.
- Render-physics live — solver writes verts → SSDF walk → render reads same frame. No sync glue introduced by refactor.
- Glimmer → tooling one-way import. Tooling never imports glimmer.

## Anti-patterns

| Bad | Why |
|---|---|
| Two passes in one iter (dedupe + consolidate) | Violates one-commit rule; audit delta becomes ambiguous |
| `git add src/` bulk-staging | Bundles user WIP per `cleanup-loop-no-bundle-user-WIP-rule-post-e293280` |
| Commit on `odin check src` green but `odin test src` red | Tests are the contract; check is not enough |
| Extract a helper with > 4 params | Wrong boundary; rethink instead of growing signature |
| Extract a helper with 1 caller | Premature abstraction = duplication's quieter sibling |
| Split a 600-LOC file because it "feels big" | LOC is not the trigger; responsibility-count is |
| Delete a < 7-day-old symbol | May be diagnostic scaffold mid-debug; check git log first |
| Bundle perf change with dedupe / consolidate | Perf needs isolated measurement |
| Tracker artifacts (cleanup.json / refactor.md / progress.md) | Violates CLAUDE.md desired-state rule |
| Asking the user what to clean next | Loop is self-pacing; audit has the answer |
| Cleanup on top of active debug WIP | Merge friction; skip iter, enqueue, retry next fire |
| Comment-strip a `#assert` contract | Contracts are non-negotiable |
| Re-add comments the user previously stripped | User-curation is deliberate; respect it |

## Tool routing

| Need | Tool |
|---|---|
| Audit LOC + decls + dup blocks | `mcp__plugin_context-mode_context-mode__ctx_execute` (Python script over src/) |
| Cross-file symbol refs | `Grep` |
| Vicky context | `mcp__plugin_vicky_vicky__query` + `__dql` |
| Read target file | `Read` |
| Apply edit | `Edit` (preferred) or `Write` (full rewrite when comment-strip on merge) |
| Compile check | `odin check src` via Bash |
| Test gate | `odin test src` via Bash |
| Bench (Pass 5 only) | `just bench-baseline` + `just bench` ×2 |
| Stage + commit + push | Bash (explicit file list, no bulk add) |
| Schedule next iter | `ScheduleWakeup` (delaySeconds: 1800 default) |
| Enqueue on skip | `mcp__plugin_vicky_vicky__enqueue` |

## Pacing

`ScheduleWakeup` `delaySeconds: 1800` (30 min) default. Cleanup runs on user-driven commits — no harness event to wait for. Idle ticks past the 5-minute cache window are pure overhead; 1800 s is the next-cheap point. User override for shorter cadence or one-shot mode honoured.
