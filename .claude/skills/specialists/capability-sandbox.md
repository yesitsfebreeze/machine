# capability-sandbox

**When**: designing the permission model for tools/agents/plugins.
**Why care**: ambient authority lets one bug compromise everything; capability scoping bounds blast radius by construction.

## Decision tree
- New tool needs resource X → grant capability for X only. Reason: principle of least authority.
- Cross-tool composition → pass capabilities, not raw paths/credentials. Reason: holder controls scope.
- Untrusted plugin → no ambient authority, all access via injected capabilities. Reason: makes scope auditable.
- Privilege escalation needed → explicit user approval, time-boxed. Reason: avoid TOCTOU + scope creep.

## Tradeoffs
- Pure capability model: rigorous, friction on every new feature.
- Role-based (RBAC): familiar, drifts toward "admin everywhere".
- Capability + RBAC hybrid: practical, harder to audit.

## Anti-patterns (why)
- Running tools with the user's full shell privileges: one prompt injection = total compromise.
- "Permission" that's checked once then cached forever: defeats time-boxing.
- Implicit globals (process env, cwd) as capability source: invisible scope.
