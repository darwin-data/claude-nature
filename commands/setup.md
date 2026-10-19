---
description: Install claude-nature and add the nature footprint to your status line
allowed-tools: Bash(claude-nature:*), Bash(command -v claude-nature), Bash(curl -fsSL https://raw.githubusercontent.com/darwin-data/claude-nature/main/install.sh:*)
---

1. Run `command -v claude-nature`. If it prints nothing, run
   `curl -fsSL https://raw.githubusercontent.com/darwin-data/claude-nature/main/install.sh | sh`.
   If the install fails, show its error and stop.
2. Run `claude-nature install-statusline` (or `~/.local/bin/claude-nature install-statusline`
   when the binary is not on the PATH yet) and show its output.
3. Tell the user, in their language and in two sentences at most, that the status line shows
   from the next prompt, and that `/nature-footprint:nature-report` gives the full report.
