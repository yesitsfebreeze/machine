# Authoring & registering a helper skill

## 1. Write the helper doc

Create `/.machine/skills/<name>.md`. Keep it tiny and operational — the correct
method, the gotcha, the order. No tutorials, no prose the model already knows.

Shape:

```markdown
# <name> — <one-line what this recurring task is>

**When:** <the situation that makes this task come up>

**Do this:**
1. <step / exact command>
2. <step>

**Gotcha:** <the thing that bit us — why the naive way fails>
```

Naming: short, kebab-case, matches the registry `name`. The file is `<name>.md`.

## 2. Register it in the tag cloud

Edit `/.machine/skills/registry.json`. Append an object to `helpers`:

```json
{
  "name": "run-migrations",
  "file": "run-migrations.md",
  "tags": ["migration", "migrate", "schema change", "alembic"],
  "summary": "apply DB migrations the project's way (never raw SQL)"
}
```

Fields:
- `name` — stable id, matches the doc filename stem.
- `file` — the doc filename (defaults to `<name>.md` if omitted).
- `tags` — trigger words. Single words match on word boundaries; multi-word
  phrases match as substrings. These are what the UserPromptSubmit hook scans.
- `summary` — one line shown in the injected reminder.

## 3. Choosing good tags (the reliability lever)

- Use the words a future prompt about this task will actually contain — verbs and
  nouns from the domain, not meta-words.
- Include common synonyms and the user's own phrasing.
- AVOID tags so broad they fire constantly (`code`, `file`, `fix`, `run`, `test`)
  — a tag cloud that always fires is noise and gets ignored.
- 3–8 tags is usually right. Too few misses; too many over-fires.

## 4. Verify the trigger

Sanity-check the JSON parses and a representative prompt would match:

```bash
node -e 'const r=require("./.machine/skills/registry.json");console.log(r.helpers.at(-1))'
```

Then confirm to the user: name, file, tags, and that it now triggers whenever a
prompt hits those tags.

## Lifecycle notes

- Helpers are project data under `/.machine`; `/oil` re-indexes the project layer
  but leaves authored helpers in place.
- Retire a stale helper by deleting its doc AND its registry entry in the same
  change — never leave an orphan entry pointing at a missing file.
- The friction sensor (`helper-suggest.mjs`) nudges at most once per session; the
  threshold lives at the top of that hook if it needs tuning.
```
