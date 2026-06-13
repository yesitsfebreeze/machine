# Source Cleanup

## Goal

Zero duplication, minimum file count, maximum density. Every line earns its place; every file owns one responsibility; shared logic lives in one place, referenced from many.

## Three forces (resolve in order)

1. **Dedupe wins.** Two near-identical blocks → extract shared. Always.
2. **Consolidate wins.** Two small files with overlapping responsibility → merge.
3. **Split only when forced.** A file is large AND mixes responsibilities → split by responsibility, never by line count. A large file doing exactly one thing well stays.

## Pass gate (pick ONE per cycle)

Run the audit (below), then take the first match:

1. Cross-file duplicate block (≥ 6 lines, ≥ 2 files) → **dedupe**
2. Symbol with zero references anywhere (verify by Grep across source AND tests) → **dead-code** (cheapest win)
3. Two small files sharing a responsibility, compatible imports/lifetimes → **consolidate**
4. Large file with > 1 responsibility → **split**
5. Nothing safely shippable → report findings + stop. Zero-change cycles are valid; half-safe changes are not.

## Audit

Per source file: LOC (`wc -l`/`tokei`), responsibilities (one sentence each — needing two sentences = mixed), duplicate blocks (jscpd or Grep spot-checks), unreferenced symbols, inlined logic other files would reuse. Rank: duplication desc → LOC desc. Use the project's own linter/analyzer when present (detect from the repo or /.machine/project.md).

## Hard rules

- **One atomic cleanup per commit.** One logical change, even if it touches several files. Never two passes in one cycle.
- **Gate green between steps.** Run the project's check/tests (use /gate or the commands in /.machine/project.md) after every extract/merge/delete. Never commit on red.
- **Never bulk-stage.** `git add <explicit paths>`; review `git diff --cached`; user WIP stays unstaged. If user WIP overlaps the target file, skip that target this cycle.
- **Respect user curation.** Never re-add comments or links the user previously stripped.

## Extraction discipline

- Move the block, replace all call sites, delete originals — in the same commit.
- Extracted helper takes > 4 params → wrong boundary; rethink instead of growing the signature.
- Helper ends up with 1 caller → inline it back. Premature abstraction is duplication's quieter sibling.

## Dead code

Grep every reference site (source, tests, configs) before deleting. Symbol younger than ~a week: check git log — it may be mid-feature scaffolding. Delete outright; never park dead code in a "maybe useful" file. Git remembers.

## Consolidation guards

Do not merge files with different lifetimes, sharply different dependency sets, or across architectural boundaries the project defines (/.machine/agent.md). When merging, keep invariant-bearing comments and assertions; drop section dividers and restates-the-code narration.

## Done when (per file)

One responsibility in one sentence; zero duplicate blocks ≥ 6 lines with any other file; zero unreferenced symbols; gate green.
