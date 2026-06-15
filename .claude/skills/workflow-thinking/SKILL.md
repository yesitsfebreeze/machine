---
name: workflow-thinking
description: >
  Structured step-by-step deep analysis via the --deepthink flag, performed with
  Claude's native extended reasoning. Use for multi-step analysis or architecture
  decisions.
license: Apache-2.0
compatibility: Designed for Claude Code
allowed-tools: Read, Grep, Glob
effort: high
user-invocable: false
metadata:
  version: "3.0.0"
  category: "workflow"
  status: "active"
  modularized: "false"

# extension: Progressive Disclosure
progressive_disclosure:
  enabled: true
  level1_tokens: 100
  level2_tokens: 3000

# extension: Triggers
triggers:
  keywords: ["deepthink", "deep analysis", "complex problem", "architecture decision", "technology selection", "trade-off", "breaking change"]
  phases:
    - plan
  agents:
    - manager-strategy
    - manager-spec
---

# Structured Deep Analysis (--deepthink)

Structured, revisable step-by-step reasoning performed with Claude's native
extended reasoning. No external tool is required — `--deepthink` raises the rigor
and visibility of the reasoning, it does not call an MCP server.

## Two Reasoning Modes

| Mode | Trigger | Mechanism |
|------|---------|-----------|
| `--deepthink` | Explicit `--deepthink` flag | Native extended reasoning, written out as an explicit, numbered, revisable thought chain |
| `ultrathink` | Keyword or auto-detection | Native extended reasoning at high effort |

**Rules:**
- `--deepthink` → produce a visible, structured thought chain (numbered steps,
  explicit revisions, explicit branches) before the conclusion.
- `ultrathink` → maximize native reasoning effort; structure is optional.
- They compose: `ultrathink --deepthink` means maximum effort AND a written,
  structured chain.
- Let depth adapt to task complexity; do not pad simple problems.

## Activation Triggers (--deepthink)

Produce a structured thought chain when `--deepthink` is present, or when the task
involves:

- Breaking down complex problems into steps
- Planning and design with room for revision
- Architecture decisions that affect 3+ files
- Technology selection between multiple options
- Performance vs maintainability trade-offs
- Breaking changes under consideration
- Multiple approaches to the same problem
- Repetitive errors

## Method

Write the reasoning as an explicit chain so the conclusion can be reviewed:

1. **Estimate scope** — state how many steps you expect.
2. **Number each step** — one move per step (frame, decompose, evaluate, decide).
3. **Revise openly** — when a later step contradicts an earlier one, write a
   revision step that names the step it corrects.
4. **Branch when alternatives exist** — label each alternative and compare them
   explicitly rather than silently picking one.
5. **Conclude** — the final step states the answer and cites the step numbers
   that support it.

## Guidelines

1. Start with a reasonable step-count estimate and adjust as needed.
2. Make revisions explicit; do not silently overwrite earlier reasoning.
3. Keep the step sequence intact and readable.
4. Only conclude once the chain is complete.
5. Use explicit branches when exploring alternatives.

<!-- machine:evolvable-start id="rationalizations" -->
## Common Rationalizations

| Rationalization | Reality |
|---|---|
| "I can think through this in my head, it is simple" | Simple problems often hide second-order effects. A written chain forces you to enumerate them. |
| "The thinking steps are just internal, I do not need to record them" | Unrecorded reasoning cannot be reviewed. The chain is the evidence for the conclusion. |
| "I already know the answer, the framework is overhead" | Confirmation bias skips disconfirming evidence. The framework forces you to consider alternatives. |
| "Branching is overkill for this decision" | Decisions with more than one viable path benefit from explicit branch comparison, even briefly. |

<!-- machine:evolvable-end -->

<!-- machine:evolvable-start id="red-flags" -->
## Red Flags

- Architecture decision made without a documented reasoning chain
- Chain ended before reaching a conclusion
- No revision steps despite encountering contradictions
- No branching when two or more viable alternatives were identified

<!-- machine:evolvable-end -->

<!-- machine:evolvable-start id="verification" -->
## Verification

- [ ] Thinking chain has a clear step-count estimate that was met or revised
- [ ] Final step states a conclusion
- [ ] At least one revision step exists if contradictions were encountered
- [ ] Branching used when multiple alternatives were identified
- [ ] Conclusion references specific step numbers as supporting evidence

<!-- machine:evolvable-end -->
