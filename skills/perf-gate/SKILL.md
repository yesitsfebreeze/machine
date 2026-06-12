---
name: perf-gate
description: Performance gate for gfx/shader work. Capture baseline, change, capture after, report delta. Use before any non-trivial rendering change.
when_to_use: Performance-sensitive rendering work, esp. shader changes. Skip non-rendering, trivial, perf-not-concern.
triggers:
  files: ["src/shader.wgsl", "src/main.rs", "src/rendering.rs", "src/ui.rs", "assets/stack.stk"]
  keywords: ["performance", "fps", "slow", "profile", "optimize", "shadow cost", "displacement cost", "lighting cost"]
auto_suggest: true
tags: [skill, material, shading, graphics, code, pipeline]
---

# Performance Gate

## Contract

Every rendering/shader task:
1. Capture **before** baseline
2. Implement change
3. Capture **after** baseline
4. Report delta

Done requires: app run check + compile check + screenshot artifact + perf artifact.

## Pre-work

```bash
cargo check
cargo run -- --perf      # stops frame 6000, writes profiler CSV, screenshot, exits
python skills/perf-gate/scripts/update_baseline.py
```

`update_baseline.py` writes `profiling/perf_baseline.csv` — two rows: `latest` (newest avg), `previous` (prior avg). Screenshot → `screenshots/frame-6000-*.png`.

Custom stop frame (early-frame regressions): `cargo run -- --stop-frame=1200`.

## Decision rule

Read `profiling/perf_baseline.csv`, call out: `full_ms`, `lighting_ms`, `shadow_ms`, `displacement_ms`, `avg_step_ms`. Verify newest screenshot valid + clean exit. Pick dominant cost first (highest ms).

## During optimization

- Narrow scope: one bottleneck at a time
- After each meaningful change: compile + perf run + baseline update
- Report delta vs `previous` (ms + %)

## Done criteria

- Latest improves target without major regression elsewhere
- `perf_baseline.csv` updated, attached in summary
- Before/after screenshot paths in summary
