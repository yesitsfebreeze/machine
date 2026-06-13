---
name: personas
description: Run the project's persona review panel in parallel against the last completed feature or fix. The panel is defined by /.proj/personas/ — each reviewer critiques from their own lens, then a synthesis pass identifies the top cross-cutting concerns and a ship verdict.
---

# Persona Panel Review

You are orchestrating the project's review panel. The user just completed a
feature or fix. Run every persona in parallel, then synthesize. The panel is
**data-driven** — it is defined by the files in `.proj/personas/`, not hard-coded
here, so it stays in sync as personas are added or revised.

## Step 1: Load the panel + the work under review

1. Read `.proj/personas.md` (the index) and **every** file in
   `.proj/personas/`. Each file is one reviewer: its name, role, what it
   scrutinizes, its standing concerns, and the question it always asks.
2. Read the project identity line and overarching goal from `/.proj/project.md`
   (and `/.proj/agent.md` if present). Capture as `{PROJECT_IDENTITY}` and
   `{PROJECT_GOAL}`.
3. Identify the feature/fix just completed from conversation context (the last
   major assistant response describing finished work). Capture it as
   `{WORK_DESCRIPTION}` — include the files touched and the user-visible effect.

If `.proj/personas/` is empty or `/.proj/` is missing, tell the user to run
`/oil-me` (or author personas) and stop — do not invent personas.

## Step 2: Spawn one subagent per persona, in parallel

For each persona file loaded in Step 1, spawn a subagent with this prompt,
substituting the persona's own definition and the work under review:

```
You are "{PERSONA_NAME}", {PERSONA_ROLE} on this project ({PROJECT_IDENTITY};
overarching goal: {PROJECT_GOAL}).

Your lens — what you scrutinize and the standing concerns you carry:
{PERSONA_BODY}

Work just completed:
{WORK_DESCRIPTION}

Review it strictly from your lens. Give exactly 4 bullets, most urgent first.
Cite real files/mechanisms (not generic advice). Where the change touches the
overarching goal, say how. End with the one direct question your persona always
asks. No praise — flag real problems only.
```

All personas run simultaneously.

## Step 3: Synthesize

After every persona responds, run a synthesis subagent:

```
You are synthesizing the project's persona panel. Each reviewer critiqued a
completed change from their lens:

{FOR EACH PERSONA: - {PERSONA_NAME} ({PERSONA_ROLE}): {PERSONA_OUTPUT}}

Produce:
1. **Top 3 concerns** — issues multiple reviewers flagged or that carry the
   highest risk on the project's own quality axes.
2. **Quick wins** — easy fixes multiple personas agree on.
3. **Unresolved questions** — genuine disagreements worth a decision.
4. **Ship verdict** — SHIP / SHIP WITH CAVEATS / DO NOT SHIP, one-line reason.

Be direct. No praise. Real problems only.
```

## Output Format

```
=== PERSONA PANEL ===

[{PERSONA_NAME}] {PERSONA_ROLE}
• ...
? ...

(one block per persona file, in the order listed in personas.md)

=== SYNTHESIS ===
Top concerns: ...
Quick wins: ...
Open questions: ...
Verdict: SHIP / SHIP WITH CAVEATS / DO NOT SHIP — reason
```

Keep each persona block under 8 lines, synthesis under 12. Scannable in 60s.
