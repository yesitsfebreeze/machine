# secrets

**When**: storing API keys, tokens, credentials; choosing env vs keyring vs vault; planning rotation.
**Why care**: leaked secrets are the cheapest attacker path; blast radius scales with where the secret lives.

## Decision tree
- Local dev → OS keyring or `.env` outside repo. Reason: prevents accidental commits.
- CI → ephemeral, masked secrets from the CI vault. Reason: no long-lived material on disk.
- Prod → centralized vault with short-lived tokens. Reason: rotation + audit trail.
- Multi-tenant blast risk → per-tenant scoping, not shared root. Reason: contains compromise.

## Tradeoffs
- Env vars: simple, easy to leak via logs/dumps/child processes.
- Keyring: OS-protected, less portable.
- Vault: best posture, operational overhead, single point of failure.

## Anti-patterns (why)
- Secrets in source / commit history: forever leaked once pushed.
- Logging request bodies that contain secrets: ends up in centralized logs, indexed.
- Long-lived static tokens: rotation becomes "never"; one leak = unbounded exposure.
