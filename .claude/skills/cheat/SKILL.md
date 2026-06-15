---
name: cheat
description: >
  Cheatsheet lookup via cht.sh — the tmux + cht.sh + fzf tool. Ships a `cht`
  script with an interactive fzf picker (topic, then query) and a direct mode
  (`cht python reverse a list`, `cht tar extract`). Use when the user wants a
  quick syntax reminder, a command flag, an idiom, or a one-liner for a
  language or CLI tool. Trigger: "/cheat", "cheatsheet for X", "how do I X in
  <lang>", "cht.sh <topic>", "remind me the flags for <command>".
metadata:
  version: "1.0.0"
  category: "tooling"
  status: "active"
  updated: "2026-06-15"
  tags: "cheat, cht.sh, fzf, tmux, cheatsheet, reference, snippets"
---

# Cheat — cht.sh cheatsheet lookup

A self-contained reimplementation of the tmux + cht.sh + fzf cheatsheet tool.
The `cht` script (next to this file) is the whole tool; this document is how to
use it and how to wire the optional tmux popup keybind.

## When the user asks for a cheatsheet

Run `cht` in **direct mode** and show the result — do not guess the syntax from
recall when a ground-truth lookup is one call away (machine law: ground truth
over recall).

- Language idiom: `cht <lang> <query...>` → `cht python reverse a list`
- Command usage: `cht <command> <query...>` → `cht tar extract gz`
- Bare topic: `cht awk` (returns the tool's top cheatsheet)

The script picks the right cht.sh route automatically: known programming
languages use `host/<lang>/<query>`, everything else is treated as a command
and uses `host/<command>~<query>`. Output is fetched with `?qT` (quiet, no
color codes) so it captures cleanly into context.

Quote the returned snippet back to the user; add a one-line note only if the
result needs caveats (deprecated flag, version-specific, platform difference).

## Interactive mode (the user's terminal)

With no arguments, `cht` runs the video's flow: an fzf picker over a curated
list of languages and CLI tools, then a `query:` prompt, then the paged result.

```
cht
```

## Optional: the tmux popup keybind

For the one-keystroke experience from the video, add a binding to
`~/.tmux.conf` that opens `cht` in a tmux popup (modern tmux) or a new window
(older tmux). Point it at wherever the user installs the script — e.g. a
symlink on PATH:

```
ln -s "$PWD/.claude/skills/cheat/cht" ~/.local/bin/cht
```

`~/.tmux.conf` (popup, tmux >= 3.2):

```
bind-key -r i display-popup -E -w 80% -h 80% "cht"
```

`~/.tmux.conf` (new window, older tmux):

```
bind-key -r i run-shell "tmux neww cht"
```

Reload with `tmux source-file ~/.tmux.conf`. Then `prefix + i` opens the
picker.

## Dependencies

- `curl` — required (the lookup)
- `fzf` — required only for interactive mode
- `tmux` — only for the keybind
- A pager (`less`) — interactive output; override with `CHT_PAGER=cat`

## Configuration

- `CHT_HOST` — alternate cht.sh host or a self-hosted instance (default `cht.sh`)
- `CHT_PAGER` — pager for interactive results (default `less -R`)

## Notes

- cht.sh is a shared free service and occasionally returns 500s under load —
  retry, or self-host and set `CHT_HOST`.
- Add or trim the `languages` / `utils` lists in `cht` to match the stack the
  user actually works in.
