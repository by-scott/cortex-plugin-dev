---
description: Review recent changes for unnecessary complexity, then fix
when_to_use: After completing a task, to clean up and simplify before delivery
required_tools:
  - dev
  - read
  - edit
tags:
  - quality
  - refactor
activation:
  input_patterns:
    - (?i)(simplif|clean.?up|refactor|优化|简化)
---

# Simplify

${ARGS}

## Identify

Run `git_diff` to see recent changes. For each changed region: could this be simpler without losing correctness?

## Detect

Abstractions that serve only one call site. Error handling for conditions that cannot occur. Comments restating the code. Defensive checks the type system already prevents. Variables used once on the next line. Wrapper functions adding no logic. Copy-paste that should be a loop or shared function.

## Fix

Apply each simplification as a minimal, targeted edit. One concern per change. Verify the simplified code compiles and passes tests.

## Report

List what was simplified and why. If something looked complex but was justified, say so — not every pattern needs simplifying.
