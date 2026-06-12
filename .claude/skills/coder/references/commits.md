# Commits — incremental, bounded, well-described

Unit of review and rollback. Small, self-contained, explained.

## Bounded

One commit = one logical change. If description needs "and", split.

- Test + impl for one behavior → one commit
- Refactor enabling feature → separate commit before feature
- Formatting/rename sweeps → own commit, never mixed with logic
- Dependency bumps → isolated

Commit reviewer can't understand alone = too big.

## Incremental

Commit after each green-refactor cycle. Don't batch a day's work.

- Bisect finds exact breaking change
- Revert surgical (one commit, not a week)
- Review fits working memory
- Stay inside feedback loop, not outrunning headlights

Rule: can't explain in one sentence = too big.

## Well-described

Conventional Commits:

```
<type>(<scope>): <subject>

<body — why, not what>

<footer — refs, breaking>
```

- **Subject** ≤50 chars. Imperative ("add", "fix"). No trailing period.
- **Body** only when *why* not obvious. Motivation, constraint, non-obvious tradeoff. Wrap 72 chars.
- **Footer**: `Refs: #123`, `BREAKING CHANGE: ...`, co-authors.

Types: `feat`, `fix`, `refactor`, `perf`, `test`, `docs`, `chore`, `build`, `ci`, `style`.

## Subject discipline

- `update stuff` → `fix(auth): reject expired tokens at boundary`
- `wip` → `refactor(walk): extract path-resolver into deep module`
- `more tests` → `test(walk): cover symlink loop edge case`
- `fixed bug` → `fix(cache): clear entry on TTL exceed, not on access`

## Body discipline

Skip when subject + diff self-explanatory.

Write when:
- *Why* not in code
- Non-obvious tradeoff
- Workaround for specific bug
- Outside-repo constraint (RFC, vendor quirk, deadline) shaped choice

Don't:
- Restate diff
- Narrate session ("tried X, then Y, settled on Z")
- Reference current task that rots ("for ABC-42 demo Tuesday")

## Anti-patterns

- Mega-commit at end of day — loses bisect, blocks review
- Mixed concerns ("fix + reformat + rename") — hides fix
- Refactor inside feature commit — reviewer can't tell
- Vague subjects (`wip`, `update`, `fix stuff`) — useless in `git log`
- Body paraphrasing diff — describe *why*, diff shows *what*
- Skipping commit between green and refactor — lose known-good checkpoint

## Amend vs new

Amend only most recent commit, only before push, only same logical unit (typo, missed file). Never amend pushed commits unless user asks. Otherwise: new commit. `fix:` after `feat:` is honest history.
