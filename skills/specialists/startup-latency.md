# startup-latency

**When**: CLI tools, short-lived processes, first-paint metrics for interactive apps.
**Why care**: cold start is the dominant cost when the process is invoked frequently; users perceive startup as overall app speed.

## Decision tree
- Work needed for every invocation → eager in main. Reason: deferral pays nothing.
- Work needed only sometimes → lazy / on first use. Reason: cuts cold path.
- Heavy init shared across invocations → daemon / persistent process. Reason: amortize once.
- Many small modules at boot → measure import/link cost. Reason: often the silent killer.

## Tradeoffs
- Lazy init: fast start, occasional first-use stalls.
- Daemon mode: instant subsequent calls, more ops complexity.
- AOT compilation / precompiled artifacts: fast start, slower build / bigger binary.

## Anti-patterns (why)
- Connecting to optional services at boot: blocks startup on something the user may not use.
- Loading config from N files synchronously: serial latency.
- Initializing logging / telemetry inline before anything else can run: first-byte delay.
