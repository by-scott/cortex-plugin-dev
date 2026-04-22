---
description: Implement a requested code change end-to-end with scoped edits and verification.
when_to_use: Use when the user asks to add, change, or fix behavior in a codebase.
required_tools:
  - project_map
  - read_file
  - replace_in_file
  - write_file
  - diagnostics
tags:
  - coding
  - implementation
activation:
  input_patterns:
    - (?i)(implement|add|build|fix|修复|实现|增加)
---

# Implement

${ARGS}

## Orient

Map the project before editing. Identify the smallest files and symbols that own the behavior.

## Change

Edit the narrowest viable surface. Prefer precise replacements over full rewrites. Preserve user changes and existing conventions.

## Verify

Run the most relevant test or diagnostic path. If verification cannot run, report the blocker and the residual risk.

## Report

State what changed, what was verified, and what remains.
