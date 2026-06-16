// Shared preamble for Stop / SubagentStop hooks: read the JSON payload from
// stdin and load the session transcript. Exits 0 (no-op) when there is nothing
// to act on -- no input, malformed JSON, a re-entrant stop, or an unreadable
// transcript. Returns { data, transcript } otherwise.
import { readFileSync } from "fs";

export function loadStopHook() {
  let raw = "";
  try {
    raw = readFileSync(0, "utf8"); // fd 0 = stdin, works on all platforms
  } catch {
    process.exit(0);
  }
  if (!raw.trim()) process.exit(0);

  let data;
  try {
    data = JSON.parse(raw);
  } catch {
    process.exit(0);
  }

  // Already fired once this stop cycle - do not loop.
  if (data.stop_hook_active === true) process.exit(0);

  const transcriptPath = data.transcript_path;
  if (!transcriptPath) process.exit(0);

  let transcript;
  try {
    transcript = readFileSync(transcriptPath, "utf8");
  } catch {
    process.exit(0);
  }

  return { data, transcript };
}
