---
description: Manage context pressure during large codebase work without losing critical state.
when_to_use: Use when a task spans many files, long logs, repeated tool calls, or model context pressure is rising.
required_tools:
  - brief
  - project_map
  - symbol_search
  - read_file
tags:
  - context
  - planning
activation:
  input_patterns:
    - (?i)(context|budget|compact|long task|上下文|压缩|长任务)
---

# Context Budget

${ARGS}

## Keep

Preserve active goals, constraints, changed files, unresolved decisions, verification state, and user preferences.

## Drop

Summarize repeated logs, dependency download noise, stale alternatives, and file contents already transformed into decisions.

## Retrieve

Use symbols and targeted reads instead of rereading whole files. Reconstruct context from durable project artifacts when possible.

## Handoff

Before compression or long transitions, write a concise brief with next action, blockers, and validation commands.
