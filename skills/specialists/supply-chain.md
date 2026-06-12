# supply-chain

**When**: adding dependencies, updating lockfiles, choosing vendor vs registry, setting audit cadence.
**Why care**: most recent high-impact breaches start with a compromised package; lockfile drift hides the true bill of materials.

## Decision tree
- Production dep → pin via lockfile, audit on update. Reason: reproducibility + intrusion detection.
- Rarely updated, security-critical → vendor. Reason: full control of the source.
- Frequent updates from trusted org → registry with verified publisher. Reason: ergonomics vs paranoia balance.
- New dep proposal → review maintainer count, release cadence, install scripts. Reason: lone-maintainer + install hooks is the classic compromise vector.

## Tradeoffs
- Strict pinning: reproducible, falls behind on patches.
- Floating versions: latest fixes, supply-chain risk.
- Vendoring: max control, audit burden on you.

## Anti-patterns (why)
- `npm install` in CI with no lockfile: non-reproducible, vulnerable to upstream takeover.
- Auto-merging dependency PRs without review: defeats the audit step entirely.
- Treating transitive deps as someone else's problem: they run your code.
