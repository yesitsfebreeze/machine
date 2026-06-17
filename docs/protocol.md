# Protocol

# Core
- kern (splitting boradcastable database that learns over time)
- git-fs ('fileless' storage inside git, quick, and full audit trail)
- mesh (cross agent communicator)
- psaido (pseudo ai code language) — SHELVED, not in the active flow; plans are a plain markdown brief
- board (taskboard)
- mine (list of useful skills and tools that can be installed on demand)
- personas

The idea of the workflow is:

# brainstorm: (1 chat, brainstorms ideas with you constantly, just /clear to start new idea)
 - uses drill to ask questions until scope of feature is well defined

# /promote (promotes findings from the brainstorm into tickets)
 - each ticked ends up in the provided ticket board under its {cwd}
 - then the next step (plan) i executed for each ticket, and repeated until approved.

# parallel:
  ## plan (dispatched fleet)
   - plans the implementation of the task
   - aware when we superseed
   - needs to rip old implementation if superseeded
   - writes the plan as a plain markdown brief the miner reads directly
   - raises questions instantly with the hub to the questioneer
   - waits until ready to proceed again

  ## implement:
   - implements approved plans using git-fs in the current orchestrators worktree
   - turns the brief into real requested code.
   - moves its ticket into doing on the taskboard
   - raises questions instantly with the hub to the questioneer
   - waits until ready to proceed again

# smooth until you need to react:
 - after the shape is agreed, plan -> implement -> gate-green run autonomously
 - ONE consolidated advisory review (personas + codex) on the finished diff, scaled to size
 - the drill surfaces only to land into main, plus on a blocker it cannot resolve

# questioneer: (single ongoing chat, ask you to resolve questions of the plans and imeplementations)
 - since we have the taskboard, we have a list of plan questions and implementation questions
 - we can gather an ongoing context of what we need to do, what the most **important** descicions are.

# /oil
Smears the machine, goes over the repository, checks if everyhing is in order.
uses /mine and other tools at disposal to keep the machine smooth and running as best as can be
for this particular repo.
* suggest to use /loop 60min /oil

