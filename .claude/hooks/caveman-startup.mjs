#!/usr/bin/env node
// SessionStart hook: activate caveman comm mode by default.
// Emits additionalContext instructing the agent to invoke the caveman skill.
// Disable: remove this hook from settings.json, or tell the agent "stop caveman".

const context = [
  "Caveman comm mode is ON by default this session.",
  "Invoke the `caveman` skill and follow it for all responses (default level: full).",
  "Off only on explicit user request: \"stop caveman\" / \"normal mode\".",
].join(" ");

process.stdout.write(
  JSON.stringify({
    hookSpecificOutput: {
      hookEventName: "SessionStart",
      additionalContext: context,
    },
  }),
);
