export CLAUDE_CONFIG_DIR := justfile_directory()

[no-cd]
default *args:
    claude --permission-mode auto --{{args}}

where:
    @echo "CLAUDE_CONFIG_DIR = {{justfile_directory()}}"
