---
name: improve
description: Rates each code file 1-10 across quality categories, rolls those ratings up into a folder tree so the parts of the app needing the most work surface, then improves worst→best. Uses compute (bash/analysis tools) to measure objective categories when it sharpens a rating. Accepts an optional target (path, module, or concern) to scope the sweep. Manual invocation only. Triggers on `/improve`, "improve code", "code review", "refactor", "optimize".
---

# Overarching Goal
Create as little and as small files as possible while keeping functionality and readability intact.
You leave the codebase cleaner, clearer, and more maintainable than you found it.
You do not add features, only polish existing code.
You do not delete code unless it's dead or redundant.

# Artifacts

json -> @.machine/improve.json

# Target (optional)

A target may be supplied as the skill argument (e.g. `/improve src/gnn`, `/improve error handling`).
- **Given** → resolve to a concrete file set (path prefix / subsystem dir / files where the concern applies), rate only those, record it under `"target"`, prune the tree to that subtree.
- **None** → sweep every integrated part of the app.

# Compute-use

Measure, don't guess, for objective categories when a tool exists and the file is big/important enough to warrant it. Detect the project's toolchain first; use what's there, install only if cheap; degrade to a judged rating if a tool is missing (and note it in `"measured"`). Skip tooling on trivial files. Re-run the same checks when re-rating.
- LOC → `cloc`/`tokei`/`wc -l` · Complexity → `lizard`/`radon`/eslint · Coverage → project test runner · Duplication → `jscpd` · Security → `bandit`/`npm audit`/`semgrep` ·
Style → project linter

# Folder rollup

After every file is rated, aggregate **bottom-up**: a folder's per-category score = mean of its subtree's files (LOC-weighted), overall `score` = weighted mean of those (lower = more work), `weakest` = lowest category. Then rank folders + files worst→best — that's the "where's the debt" answer and the improvement order.

# Workflow

1. Resolve scope → file set.
2. Rate each file 1-10 per category (measure where it helps). Write `rating`, `score`, `measured`, top-3 `todos`.
3. Build the folder `tree` + `ranking`.
4. Set `"indexed": true`.
5. Improve worst→best (worst folder, its worst file first): fix, then **delete that file's entry from `files`** (drop it from `tree`/`ranking` too). Re-rate only if you leave it unfinished — otherwise it's gone. While improving, strip unnecessary comments: keep only those a competent reader would need to avoid reading the code twice to understand it; delete comments that merely restate what the code plainly says. Never strip docstrings, public-API doc comments, or license headers.
6. Small commit per file improved. Repeat until budget exhausted.

**The json only holds outstanding work.** A finished file is deleted, not marked done — so the file shrinks over time and a clean codebase converges to an empty `files` list. Never accumulate `status:"done"` records or completed-work logs.

Set a goal via `/goal`, or hand the user this:
```
/loop 10m Go over <TARGET>. Rate each code file 1-10 per category, measuring LOC/complexity/coverage/duplication/security/style with tools where it sharpens the rating (skip on trivial files). Write rating, score, and top-3 todos to the json. Roll up into a folder tree with per-category aggregates; rank folders+files worst→best. Once everything is indexed, set "indexed":true and improve from worst folder to best, worst file first. After improving a file, delete its entry from the json (and from tree+ranking) so the file shrinks toward empty, then move on. Commit after each iteration.
```

## The json

```json
{
 "target": null,            // null = whole-codebase sweep
 "indexed": false,          // true once every in-scope file is rated AND the tree built
 "rating": {
  "min": 0, "max": 10,
  "categories": ["loc", "complexity", "..."],
  "weights": { "security": 2 }   // optional per-category; default 1
 },
 "files": [{                // outstanding work ONLY; delete an entry once its file is improved
  "path": "src/gnn/fusion.ts",
  "rating": [0.5, 1, "..."],     // one per category, SAME ORDER as categories
  "score": 3.2,                  // weighted mean; lower = more work
  "measured": ["loc", "complexity"], // computed vs. judged
  "todos": ["...", "..."]        // max 3, by impact
 }],
 "tree": {                  // aggregated bottom-up
  "path": ".", "score": 4.1,
  "categories": { "loc": 5.0, "complexity": 3.4 },
  "weakest": "complexity", "fileCount": 42,
  "files": ["src/index.ts"],
  "children": [ /* same shape */ ]
 },
 "ranking": { "folders": ["src/gnn", "..."], "files": ["src/gnn/fusion.ts", "..."] } // worst→best
}
```

## Categories

Pick the ones that apply; keep `categories` consistent across the run so arrays and the rollup line up:
- LOC (lines of code)
- Complexity (cyclomatic complexity)
- Clarity (subjective, based on readability, naming, structure)
- Comments (presence and quality of comments)
- Single responsibility (does the file have one clear purpose?)
- Adherence to requirements (does the file meet the specified requirements?)
- Test coverage (are there sufficient tests for the file?)
- Performance (are there any performance issues in the file?)
- Security (are there any security vulnerabilities in the file?)
- Maintainability (how easy is it to maintain and update the file?)
- Modularity (is the file modular and reusable?)
- Documentation (is there sufficient documentation for the file?)
- Code style (does the file adhere to the project's coding style guidelines?)
- Dependency management (does the file manage its dependencies effectively?)
- Error handling (does the file handle errors gracefully?)
- Code duplication (is there any duplicated code in the file?)
- Scalability (can the file handle increased load or complexity?)
- Extensibility (can the file be easily extended with new features or functionality?)
- Testability (is the file designed in a way that makes it easy to test?)
- Code smells (are there any code smells present in the file?)
- Best practices (does the file follow best practices for the language and framework?)
