# Patch-Queue Drain Loop

## Goal

Drain `.patches/` freshest-first. Each pending patch is re-validated against the
current `HEAD`, then shipped (one clean commit on master) or rejected (moved to
`.patches/rejected/` with a reason). The producer side is the perf loop
(`prompts/loop.md`) and any agent that drops a patch + sidecar per
`.patches/README.md`. This loop is the consumer.

The queue contract is authoritative in `.patches/README.md`. Read it first.

## Loop pattern (every iter)

```
PICK → VALIDATE → SHIP|REJECT → RESTART
```

1. **PICK.** Vicky-first. List pending patches (`.patches/*.patch` with a matching
   `.json`, excluding `applied/` + `rejected/`). Sort freshest-first: highest `id`,
   then newest `created`. Take the head.
2. **VALIDATE.** Apply against `HEAD`, run gates (invariants → odin check → bench
   if `needs_bench` → odin test).
3. **SHIP | REJECT.** Green → commit (explicit file stage) + push + move to
   `applied/`. Any gate fails → revert working tree + move to `rejected/` with the
   reason. Never leave master broken.
4. **RESTART.** `HEAD` moved; re-derive. Schedule next iter, name the next patch id.

One patch per iter. A rejected patch never blocks the queue.

## Phase decision gate (every iter)

1. **Empty queue.** No pending patch → enqueue a vicky note "patch queue drained"
   + exit clean. Zero-write iter is a red flag only when the queue was non-empty.
2. **Concurrent-run gate.** Live `glimmer.exe` / `bench.exe` → skip iter, reschedule.
   A bench mid-run contaminates measurement.
3. **WIP gate.** `git status` shows user code WIP overlapping a patch's `target` →
   the apply will likely conflict; that is expected and handled by the staleness
   path (reject `stale`). Never bulk-stage user WIP.
4. **Malformed patch.** `.patch` without sidecar, or sidecar missing required
   fields → move both to `rejected/` with `malformed`, continue.

## Per-patch validation protocol

Let `P = NNNN-slug`, `T = sidecar.target`, `S = sidecar.target_section`.

```
0. Read .patches/P.json. Validate required fields.

1. Staleness check:
     git apply --check .patches/P.patch
   Fail → git apply --3way --check (needs base_sha reachable).
   Still fail → REJECT "stale: git apply failed vs $(git rev-parse --short HEAD)".

2. Capture baseline (only if needs_bench):
     just bench-baseline          # baseline = HEAD, BEFORE applying

3. Apply:
     git apply .patches/P.patch   # or --3way if step 1 needed it

4. Invariant gates (for each entry in invariants_touched):
     gpu-struct-layout / wgsl-odin-mirror → odin check src must pass the
       package-scope #assert size_of/offset_of gates
     reverse-z / soa-aos / tet-as-atom → confirm the diff did not violate
       (read the hunk; these are review gates, not compiler gates)

5. Compile gate:
     odin check src      → red ⇒ git checkout -- T ; REJECT "odin check red"

6. Bench gate (only if needs_bench):
     just bench ; cp bench-out/bench.report.json bench-out/run1.json
     just bench ; cp bench-out/bench.report.json bench-out/run2.json
     Compare section S: candidate.gpu_us_mean vs baseline.gpu_us_mean.
       Both batches > +2% on S            → REVERT + REJECT "regressed"
       Untouched sections inflate uniformly (resolve/tonemap/entry all up,
         frame_total CPU climbing)        → thermal contamination, NOT a
         regression. Idle ~30 min, re-bench. Do not reject on this signature.
       Within ±2% / win                   → proceed
     Record measured vs sidecar.expected_delta.

7. Test gate:
     odin test src       → red ⇒ git checkout -- T ; REJECT "tests red"

8. SHIP:
     git add T            (explicit; never git add src/)
     git diff --cached    (review — no stray user WIP bundled)
     git commit           (subject from sidecar.title; body cites measured delta)
     git mv .patches/P.patch .patches/applied/  (and P.json)
     git add .patches/applied/  ; fold into the same commit or a follow-up
     git push origin master
```

REJECT = move `P.patch` + `P.json` into `.patches/rejected/`, append a one-line
reason to the sidecar (`"reject_reason": "...", "reject_sha": "<HEAD>"`), commit
the move (kind: chore). The rejected patch is a record, not a deletion.

## Commit format

`feat(...)` / `perf(...)` / `fix(...)` / `refactor(...)` / `cleanup(...)` per
`sidecar.kind`. Perf bodies cite the measured section delta (mean + p95, both
batches if cited). Mechanical / cleanup bodies state "runtime bit-identical" and
skip the section table. Always append:

```
Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
```

## Hard rules

- **Vicky-first** before any reasoning on a patch's target topic.
- **Freshest-first.** Newest patches apply cleanly more often; draining oldest-first
  maximizes staleness rejects.
- **Re-derive after every ship.** `HEAD` moved → the next patch's baseline + apply
  context shifted.
- **Single-instance gate.** No bench while another `glimmer.exe` / `bench.exe` runs.
- **No bundling user WIP.** Explicit file stage, pre-commit `git diff --cached`.
- **Never leave master broken.** Revert on any red; the queue absorbs the loss.
- **Reject is first-class.** A stale / regressed / red patch is logged in
  `rejected/`, never silently dropped, never left to block the head.

## Termination

- Queue empty across two consecutive iters → exit, enqueue "queue drained".
- Build fail / TDR persists across two iters → exit, enqueue diagnosis.
- User says "stop loop" / "pause".

## Pacing

`ScheduleWakeup delaySeconds: 1800` default (30 min) — also lets the machine cool
between bench-gated iters so thermal throttle does not contaminate the next
measurement. Shorter cadence only on explicit user override.
