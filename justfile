# Launch Claude Code with THIS repo as the .claude config dir.
#
# This repo *is* a .claude directory: its contents are the .claude layout
# (settings.json, commands/, agents/, skills/, hooks/, output-styles/, rules/)
# plus runtime state (.credentials.json, history.jsonl, projects/, sessions/).
# Setting CLAUDE_CONFIG_DIR to it makes Claude use this repo as its config dir.
#
# `just` sets the env var itself, so this is identical on Windows, macOS, Linux.
# justfile_directory() resolves the path no matter where you invoke `just`, and
# [no-cd] keeps Claude running in your current working directory.

export CLAUDE_CONFIG_DIR := justfile_directory()

[no-cd]
default *args:
    claude --permission-mode auto {{args}}

# Sanity check: print the config dir that will be used.
where:
    @echo "CLAUDE_CONFIG_DIR = {{justfile_directory()}}"
