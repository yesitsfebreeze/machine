# agent-sandbox

**When**: running model-generated code/commands, choosing isolation level, scoping fs/net access.
**Why care**: agent tools execute attacker-influenced input; default-deny on capabilities is the only safe baseline.

## Decision tree
- Code from the model → run in a sandbox process. Reason: prompt injection can craft arbitrary commands.
- File access → scope to project root + explicit allowlist. Reason: stray reads can exfiltrate secrets.
- Network → default deny; allowlist known endpoints. Reason: exfil channels are the highest-impact failure.
- Long-lived state → ephemeral by default, opt-in persistence. Reason: blast radius shrinks.

## Layers (why each)
- Process isolation: kernel-enforced; survives bugs in your code.
- Filesystem scoping: limits read+write blast radius.
- Network policy: limits exfil and remote command-and-control.
- Time/CPU limits: bound runaway agent loops.

## Tradeoffs
- Stronger sandbox: more friction, occasional false denials.
- Capability prompts: user-in-the-loop safety, interrupts flow.
- Trust-on-first-use: low friction, weak posture.

## Anti-patterns (why)
- Running tools as the user with no scope: full host compromise on prompt injection.
- "Read anywhere, write nowhere" without enforcement: relies on the model behaving.
- Permission once == permission forever: scope creep without audit.
