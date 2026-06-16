The README is understandable, but it currently reads like three documents merged together: a pitch page, install docs, and an internal architecture reference. The biggest issue is not bad content; it is repeated concepts and unclear load boundaries.

Main duplicated ideas:

The portable plugin vs per-repo project layer idea is repeated too many times. It appears in “What is this?”, the diagram, “How it works”, “Project layer”, “Clean lifecycle”, and “Layout”. This is the core concept, so keep it, but state it once in the intro and once in the architecture section. Everything else can reference it briefly.

The /oil behavior is also repeated. It “reads the repo and writes /.machine/” in the intro, quick start, lifecycle, how-it-works, project layer, and layout. One canonical explanation is enough:

/oil scans the repo and writes /.machine/: identity, stack, glossary, personas, and project rules.

The kern memory explanation appears in “What it can do”, “How it works”, “MCP servers”, “Companion plugins”, “Machine law”, and “Cross-cutting behaviors”. It should probably have one short bullet in the feature list and one reference entry in the MCP/plugin table.

The quality gate / persona review / root-cause law cluster is repeated in several forms: “quality gate”, “review panel”, “machine law”, “job lifecycle”, “cross-cutting behaviors”, “coder”, “clean”, “gate”, “personas”. This makes the system feel heavier than it may be. I would collapse the philosophy into one “Operating rules” section and keep the commands as plain reference entries.

The agent list and skill list conflict with the earlier summary. The “What it can do” section says “Four agents stay loaded” and “Seven skills load by default”, but the later “Full functionality reference” lists more core agents and eight+ core skills, including mine, improve, promote, etc. That inconsistency is the most important cleanup item. It makes the reader doubt what is actually loaded.

The marketplace/install story is confusing. Early on, it says the marketplace.json is the single Claude Code plugin marketplace and shows installing machine, git-fs, and split from yesitsfebreeze/machine. Later, it says git-fs is a companion plugin installed from yesitsfebreeze/git-fs. Those two install paths appear contradictory. Pick one source of truth.

The MCP server list is duplicated and inconsistent. In Quick Start, bundled servers are kern, mesh, context7, pdf-reader, context-mode. In the later MCP table, board appears as bundled, while kern is moved under companion plugins. That needs a hard split:

Bundled in plugin.json:
mesh, board, context7, pdf-reader, context-mode — or whatever is actually true.

Installed separately:
kern, git-fs, split — or whatever is actually true.

Right now the README says both.

Overly complex / probably unnecessary for a public README:

The full functionality reference is too long for the main README. It belongs in docs/reference.md. The public README should sell the idea, install it, explain the mental model, and point to reference docs. Tables for every hook, output style, lifecycle stage, and addon kit make the project feel harder to understand than it probably is.

The agent taxonomy is too detailed too early. A new user probably does not need to know manager-tdd, manager-ddd, builder-agent, builder-skill, builder-plugin, manager-spec, manager-strategy, etc. The useful promise is: default agent handles most work; specialists are available when needed; /drill orchestrates bigger tasks.

The emoji-heavy feature list makes the project look less serious than the architecture is. A few badges are fine, but the section headers like “🤖 Bare-bones agent core”, “🧰 Small skill core”, “📦 mine addon kit”, “🧠 Compounding memory” compete visually. For a dev-tool README, calmer headings would improve credibility.

The “full operating system” metaphor is strong but slightly inflated. It sounds cool, but also vague. “Portable Claude Code plugin layer” or “portable agent workspace” is clearer.

The orchestrator mode sounds very ambitious: background subagents, durable state files, validation, approval footers, non-blocking conversation. If this is fully working, keep it but make it concrete. If partly aspirational, soften it. Public READMEs lose trust when they overclaim.

The mine/ addon kit has too many named parts in the intro. It is enough to say: optional agents, skills, hooks, and references live in mine/ and are only slotted when useful. The full inventory can go later or into docs.

The matrix/numeric junk at the bottom should be removed entirely:

11 12 13 21 22 23 ▶31 ...

It looks accidental and damages the polish of the README.

What I would cut or move:

Move these sections to docs/reference.md:

Full functionality reference
Core agents table
Core skills table
MCP servers table
Companion plugins table
Lifecycle hooks table
Output styles table
Full mine/ inventory
Cross-cutting behaviors

Keep the README closer to this shape:

# the machine

One-sentence pitch.

## What it is

Portable Claude Code plugin.
Per-repo project layer.
Default driver + optional specialists.
Quality gate + memory + hooks.

## Install

/plugin marketplace add ...
/plugin install ...

## First run

/drill
or
/oil

Explain exactly what gets written and what stays portable.

## Core concepts

machine plugin
/.machine project layer
mine addon kit
kern memory
mesh coordination
gate/review

## Common commands

/drill
/oil
/gate
/mine
/plugin update machine

## Layout

Short tree.

## Reference

Link to docs/reference.md.

Best simplification:

Rename the main concepts into three clean layers and use them consistently:

machine/      portable plugin payload
.machine/     generated project layer
mine/         optional addon kit

Then everything else becomes an implementation detail.

The README currently has good material, but it is overexplaining. The core idea is strong: a portable Claude Code plugin that specializes itself per repo. Make that the spine. Everything that does not help a new user install, understand, or trust that idea should move out of the README.