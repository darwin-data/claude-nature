---
description: Install lamarck and add the nature footprint to your status line
allowed-tools: Bash(lamarck:*), Bash(command -v lamarck), Bash(curl -fsSL https://raw.githubusercontent.com/darwin-data/lamarck/main/install.sh:*)
---

1. Run `command -v lamarck`. If it prints nothing, run
   `curl -fsSL https://raw.githubusercontent.com/darwin-data/lamarck/main/install.sh | sh`.
   If the install fails, show its error and stop.
2. Run `lamarck install-statusline` (or `~/.local/bin/lamarck install-statusline`
   when the binary is not on the PATH yet) and show its output.
3. Tell the user, in their language and in two sentences at most, that the status line shows
   from the next prompt, and that `/lamarck:report` gives the full report.
