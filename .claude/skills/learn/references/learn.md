# Lesson Capture

## Goal

Nothing durable learned in a session is lost. Lessons land in the right store: kern for knowledge, glossary for vocabulary, project law for binding rules.

## What counts as a lesson

- A user correction ("no, X works like Y") — highest priority, capture verbatim intent
- A surprising failure and its root cause
- A decision made and its rationale
- A term defined, renamed, or disambiguated
- A constraint discovered (platform limit, API quirk, project invariant)

## Flow (per lesson)

1. **Dedup first.** `mcp__kern__query` the lesson's topic; if kern already holds it accurately, skip. If it holds it wrongly, this is an update — reuse the existing `object_id`.
2. **Route by kind:**
   - Vocabulary → add/fix a row in `/.proj/glossary.csv` (category,term,definition)
   - Binding rule candidate → propose an addition to `/.proj/agent.md` to the user; never add law silently
   - Everything else → `mcp__kern__ingest`: one coherent excerpt per lesson, with `title`, `descriptor`, stable `object_id`. Many small well-titled excerpts beat one dump.
3. **Verify.** Re-query kern for the ingested title; confirm it returns. Quote the confirmation.

## Hard rules

- Kern down (`mcp__kern__health` unresponsive) → capture glossary/law items anyway, list the kern-bound lessons in the reply so the user can re-run /learn later. Never write lesson files as a kern substitute (machine law: no file-memory for durable knowledge).
- Never duplicate a lesson into CLAUDE.md or skill bodies — single source of truth.
- Lessons are facts, not narratives: one excerpt = one claim + its why.

## Done when

Every lesson from the session is either captured (and verified by re-query), routed to glossary/law, or explicitly skipped as already-known.
