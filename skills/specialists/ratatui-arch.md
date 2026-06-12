# ratatui-arch

**When**: structuring a ratatui app, deciding state ownership, layout composition.
**Why care**: immediate-mode without discipline turns into a re-render-everything ball of mud.

## Decision tree
- Pure render → derive from `App` state. Reason: single source of truth, deterministic frames.
- Long-running work → background task + channel back to App. Reason: render loop must not block.
- Component-local state → keep inside the widget if it doesn't outlive a frame. Reason: avoids App bloat.
- Cross-component state → lift to App. Reason: components can't reach sideways.

## Tradeoffs
- Immediate mode: simpler mental model, redraws everything. Pay: diff via terminal buffer, mostly cheap.
- Stateful widgets: ergonomic for lists/tables. Pay: state lives outside the widget, must be threaded through.
- Custom widgets: full control. Pay: re-implementing layout primitives.

## Anti-patterns (why)
- Doing IO inside `draw`: blocks the frame, jitters input.
- Storing widget instances long-term: ratatui widgets are meant to be ephemeral per frame.
- Deep prop drilling instead of an app-level store: refactor when threading exceeds two levels.
