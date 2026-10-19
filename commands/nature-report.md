---
description: Nature footprint of your Claude Code sessions (energy, climate, water, basin stress)
argument-hint: "[--since YYYY-MM-DD] [--project TEXT]"
allowed-tools: Bash(claude-nature:*)
---

Run `claude-nature report $ARGUMENTS` and show its output as it is, in a code block.

If the command is not found, tell the user to run `/nature-footprint:setup`, and stop.

After the block, add two sentences at most, in the user's language: what the water figure
means next to the basin table. Do not add a figure that the output does not contain.
