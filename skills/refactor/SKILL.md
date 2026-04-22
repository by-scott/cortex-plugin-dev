---
description: Refactor code while preserving behavior and reducing structural risk.
when_to_use: Use when the user asks for restructuring, cleanup, simplification, or architecture improvement.
required_tools:
  - project_map
  - symbol_index
  - symbol_search
  - read_file
  - replace_in_file
  - diagnostics
tags:
  - coding
  - refactor
activation:
  input_patterns:
    - (?i)(refactor|restructure|cleanup|重构|整理|架构)
---

# Refactor

${ARGS}

## Boundary

Define the behavior that must not change. Locate callers before moving or renaming symbols.

## Shape

Prefer reducing coupling, narrowing responsibilities, and deleting dead paths over adding abstraction.

## Execute

Make small coherent edits. Keep public interfaces stable unless the user explicitly asks for a break.

## Validate

Run targeted diagnostics/tests and inspect the resulting diff for accidental behavior change.
