# Vault Learn Loop

## Goal

Drive `vicky/` toward **zero gaps, crystal-clear conclusions, every load-bearing invariant captured**. Each conclusion stands alone: read its frontmatter + first paragraph and you have the contract. Sources are scaffolding; conclusions are the deliverable.

The agent self-paces: searches, finds gaps, concludes, restarts. The user never has to point at a target. The loop reads vault state at every iter and picks the next move.

## Loop pattern (every iter)

```
SEARCH → FIND → CONCLUDE → RESTART
```

1. **SEARCH.** `vicky:query` the active topic + run gap DQL. Read every hit's frontmatter. The vault is the first source of truth.
2. **FIND.** Pick ONE highest-leverage gap from the audit. Either (a) a SOTA paper not yet read, (b) a cluster of un-absorbed sources missing an umbrella, (c) a chain-close opportunity, or (d) a conclusion that lacks a load-bearing invariant the source carries.
3. **CONCLUDE.** Write ONE vault deliverable: a new source, a new conclusion, a conclusion extension, or a crystalize call. Crystal-clear means: someone reading just the conclusion knows the contract without opening any source.
4. **RESTART.** Audit again. Schedule next iter at 60 s clamp. Name the next concrete target in the wake reason — the audit already surfaced it.

The pattern is unconditional. Skip a step → the iter is invalid.

## Hard rules

- **One write per iter.** Source OR conclusion OR crystalize call. Never two. The gap audit becomes ambiguous if multiple deliverables land in the same iter.
- **Vicky-first always.** No PDF fetch, no synthesis, no crystalize without running `vicky:query` + the gap DQL first.
- **≥ 2 sources per conclusion.** Per `vicky/WORKFLOW.md`. Synthesising a one-source umbrella is a hard error — defer until a sibling lands.
- **`dry_run: true` first** on every crystalize against a conclusion the agent has not absorbed into before. Confirm the move list, then run for real.
- **Self-pacing only.** Never ask the user "what should I do next". The vault state has the answer; the gate below resolves it.
- **Stop on user word.** "stop loop" / "exit loop" / "pause" → enqueue summary + exit.

## Phase decision gate (run every iter, before picking a target)

After Vicky-first, before any write:

1. **Cheap-win check.** Sources cited inline by a conclusion but not yet under `.absorbed/` → **CRYSTALIZE** (free archival pass).
2. **Cluster check.** ≥ 3 un-absorbed sources on the same sub-topic AND a viable absorber (existing conclusion OR every load-bearing claim can land in one umbrella) → **CRYSTALIZE**.
3. **Chain-close check.** Multi-part deep-read whose last part has landed but no umbrella conclusion exists yet → **LEARN** (synthesise umbrella now, before scope drift).
4. **Default → LEARN.** Fetch the next deep-read target from the gap list.

Never run both branches in one iter.

## LEARN branch

L1. **Pick ONE deep-read target.** Constraints:
   - Highest-leverage on-topic gap. Architectural sibling > tangential reference.
   - Either (a) a SOTA paper not in vault, or (b) an existing 0-inlink source ready to synthesise into a conclusion.
   - One PDF batch per iter (~20–30 pages). Multi-part deep-reads are normal.

L2. **PDF fetch** via `mcp__plugin_pdf-reader_pdf-reader__read_pdf`. Pages param mandatory for > 10 pages. If output overflows inline budget:
   - `mcp__plugin_context-mode_context-mode__ctx_execute_file` against the persisted JSON to extract page_texts.
   - Pass an `intent` string so output > 5 KB auto-indexes.
   - `mcp__plugin_context-mode_context-mode__ctx_search` pulls only matching sections.

L3. **Save extraction as vault source.** Template:
   - Title: `<AuthorYear>-<topic>-deep-read-part-N-pages-X-Y-<gist>`. Vicky truncates ~60 chars — check final filename before linking.
   - Frontmatter: `tags: [source, sota, siggraph-YYYY, <topic-tag>, deep-read-part-N]`, `related: [<wikilinks>]`.
   - Body: verbatim quotes for load-bearing claims; per-page summary table; **glimmer-applicability** subsection (what transfers, what doesn't, why); pain-matrix delta against UE5 / AAA architecture when relevant.
   - End with `## Sources` (URL + cross-refs to other parts).

L4. **Chain-close → synthesise umbrella** via `mcp__plugin_vicky_vicky__conclude`:
   - Title: `phynite-<topic>-<verdict>` (e.g. `phynite-megalights-2025-fourth-aaa-sibling-restir-reject-confirmed`).
   - Frontmatter `sources:` MUST list every part. `related:` MUST link sibling-pattern conclusions.
   - Body: split-decision table (adopt / reject / pin-as-bar / dodge), hard-rules-locked section, open-questions section.
   - If the full source cluster is also crystalize-ready → **defer crystalize to next iter**. One write per iter; next iter's gate picks CRYSTALIZE automatically.

L5. **Crystal-clarity check before writing.** A conclusion is crystal-clear iff:
   - Frontmatter `sources:` + `related:` are populated.
   - First paragraph states the verdict in one sentence.
   - Split-decision table is present (or "N/A — single verdict" stated explicitly).
   - Every load-bearing claim from every cited source has a one-liner in the body.
   - No "currently" / "today" / "not yet" / "planned" phrasing — desired-state only per CLAUDE.md.
   - If any line fails the check, fix before writing. Do not ship a foggy conclusion.

## CRYSTALIZE branch

C1. **Pick ONE crystalize target.** Priority order:
   - Source already cited inline by an existing conclusion but not in `.absorbed/`. Fastest win.
   - Sibling cluster (2–5 sources on the same sub-topic) that justifies one umbrella conclusion.
   - Orphan source with no existing conclusion — requires synthesis pass first.

   Never absorb across topic boundaries. A source belongs to exactly one umbrella; spans two → split-cite in both `related:` arrays, absorb into the primary.

C2. **Branch decision: absorb-only / synthesise-then-absorb / new-umbrella.**
   - **Absorb-only.** Conclusion covers every load-bearing claim. `mcp__plugin_vicky_vicky__crystalize(conclusion=<slug>, absorb=[<slugs>], dry_run: true)` → confirm move list → real run.
   - **Synthesise-then-absorb.** Conclusion missing claims → extend conclusion body (split-decision row / hard-rule / open-question) BEFORE crystalize. Information loss otherwise.
   - **New umbrella.** No conclusion exists → `mcp__plugin_vicky_vicky__conclude` with cluster of ≥ 2 sources in `sources:`. Title `phynite-<topic>-<verdict>`. Then crystalize the same cluster into the new conclusion.

C3. **Pre-absorb invariant check.** Before the real crystalize call:
   - Conclusion `derived_from:` array exists (init if missing).
   - Every load-bearing quote from every source has a one-liner in the conclusion. If not, back to C2 synthesise branch.
   - Source's `related:` wikilinks are mirrored in the conclusion's `related:` so graph reachability survives the move.

C4. **Crystalize.** `mcp__plugin_vicky_vicky__crystalize(conclusion=<slug>, absorb=[<slugs>], dry_run: false)`. Confirm response lists the moves into `.absorbed/`.

C5. **Rebuild graph.** `mcp__plugin_vicky_vicky__learn` (no-fetch relink) so absorbed sources drop from the live graph and remaining inlinks rewire.

## Postflight (every iter)

P1. **Gap audit.**
   - Re-run q:gap DQL + un-absorbed-source DQL.
   - LEARN win: new source/conclusion has ≥ 1 inlink.
   - CRYSTALIZE win: absorbed slugs no longer appear in the un-absorbed list.
   - Compare un-absorbed count vs previous iter. Trend MUST be flat or down. If up, the iter probably introduced a foggy conclusion or a source without absorber.

P2. **Schedule next iter.** `ScheduleWakeup`, `delaySeconds: 60` (clamp floor). Reason names the concrete next target — the audit just produced it. Examples:
   - `"deep-read part 2 of <paper> pages 24-48"`
   - `"crystalize 3 sources into phynite-<topic>-conclusion"`
   - `"synthesise umbrella phynite-<topic>-<verdict> from cluster of 4 sources"`

P3. **End-state hard rule.** Either ONE new vault write OR clean exit via enqueue with `"skipped: <reason>"`. Never both. Never neither. Zero-write iter = red flag — Vicky-first was skipped or the gap list was misread.

## Termination

Loop self-terminates when **both** hold:
- LEARN side: q:gap DQL returns only low-priority / tangential candidates (no high-leverage on-topic gap remaining).
- CRYSTALIZE side: un-absorbed-source DQL is empty OR every remaining source is tagged `pin-as-raw-reference` (canonical specs that stay accessible — e.g. wgpu spec excerpts, Si 2010 paper raw text).

On termination: enqueue final summary (iter count, LEARN writes, CRYSTALIZE calls, conclusions extended, new umbrellas, remaining un-absorbed, pinned exceptions, pain-matrix delta, next-suggested targets).

Other stop conditions:
- User says "stop loop" / "exit loop" / "pause".
- Three consecutive iters produce zero vault writes (systemic blocker — every remaining source needs synthesis effort > one-iter budget).
- Disk write fails or vicky-plugin error persists across two iters.

On any stop: enqueue the same summary block.

## Anti-patterns

| Bad | Why |
|---|---|
| Two writes in one iter (LEARN + CRYSTALIZE) | Violates one-write rule; gap audit becomes ambiguous |
| Re-extracting a PDF already in vault | Wasted iter; Glob first |
| Synthesising a conclusion without reading all source parts | Conclusion misses load-bearing invariants |
| Crystalize before every load-bearing claim is mirrored | Information loss; unique invariants vanish from live graph |
| Absorb a source into the wrong umbrella because it shares a keyword | Absorber gains claims it shouldn't carry |
| Skip `dry_run: true` on first crystalize against a new conclusion | Path collisions land destructively |
| Forget `mcp__plugin_vicky_vicky__learn` after crystalize | Graph stays stale; inlink count lies for next iter |
| One-source umbrella conclusion | Violates `min_sources_per_conclusion: 2`; defer until sibling lands |
| Crystalize sources the user just wrote (recent enqueue / remember) | May still be in flight for a different umbrella; respect ≥ 1 iter cooldown |
| Re-adding cross-link backlinks the user previously stripped | User-curation is deliberate; respect it |
| Writing current-state framing into long-lived docs | Per CLAUDE.md — desired state only in `vicky/conclusions/` |
| Asking the user what to do next iter | Loop is self-pacing; vault state has the answer |
| Hardcoding iter count or target paper in the loop prompt | Re-derive from vault state each iter |
| Treat `.absorbed/` as a trash dir | It is the derived_from provenance archive — never delete, never rename |
| Foggy conclusion: missing split-decision table or contains "currently" | Fails the L5 crystal-clarity check; rewrite before shipping |

## DQL templates

Un-absorbed phynite sources (CRYSTALIZE cluster check):

```dql
LIST file.link
FROM "vicky/sources/engine"
WHERE !contains(file.folder, ".absorbed")
SORT file.mtime ASC
```

Sources cited inline by a conclusion but not yet moved to `.absorbed/` (cheapest CRYSTALIZE wins):

```dql
TABLE file.outlinks AS sources, length(file.outlinks) AS n
FROM "vicky/conclusions/engine"
WHERE length(file.outlinks) > 0
SORT n DESC
```

Conclusions missing `derived_from:` frontmatter (need init before first crystalize):

```dql
LIST file.link
FROM "vicky/conclusions/engine"
WHERE !derived_from
```

q:gap (sources with 0 conclusion inlinks — LEARN-phase synthesis priority):

```dql
LIST file.link
FROM "vicky/sources/engine"
WHERE length(file.inlinks) = 0 AND !contains(file.folder, ".absorbed")
SORT file.mtime ASC
```

## Tool routing

| Need | Tool |
|---|---|
| Vault query / hub-spoke navigation | `mcp__plugin_vicky_vicky__query` + `__dql` |
| Find un-absorbed sources | `__dql` with the un-absorbed template above |
| Find conclusion that should absorb | `__query` + Glob `vicky/conclusions/engine/*<topic>*` |
| Save source | `__remember` (titles truncate ~60 chars) |
| Synthesise new umbrella | `__conclude` |
| Extend a conclusion before absorb | `Edit` against the conclusion file (desired-state framing only) |
| Absorb sources into conclusion | `__crystalize` (always `dry_run: true` first) |
| Rebuild graph after absorb | `__learn` |
| Fetch PDF | `mcp__plugin_pdf-reader_pdf-reader__read_pdf` (pages param mandatory for > 10 pages) |
| Large tool output overflow | `mcp__plugin_context-mode_context-mode__ctx_execute_file` + `__ctx_search` |
| Web URL discovery | `WebSearch` (load via ToolSearch if not default) |
| Schedule next iter | `ScheduleWakeup` (delaySeconds: 60, clamp floor) |

## Pacing

60 s clamp on every `ScheduleWakeup`. Pure self-driven vault build + condensation — no external work to wait on. Back-to-back chain. User override for slower pacing or one-shot mode honoured.
