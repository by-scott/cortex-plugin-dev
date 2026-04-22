---
description: Guided git commit workflow — review changes, draft message, stage, commit
when_to_use: When the collaborator asks to commit changes or when work reaches a natural commit point
required_tools:
  - dev
  - read
tags:
  - git
  - workflow
activation:
  input_patterns:
    - (?i)(commit|提交)
---

# Commit

${ARGS}

## Survey

Run `git_status` to see all changes. Run `git_diff` to understand what changed and why. Read recent `git_log` entries to match the repository's commit message conventions.

## Analyze

Classify the change: new feature, enhancement, bug fix, refactor, test, docs, or chore. Identify which files belong together in a single logical commit. Flag anything that should NOT be committed — credentials, build artifacts, unrelated modifications.

## Draft

Write a concise commit message. First line: imperative verb, what changed, why — under 72 characters. Body only when the diff cannot convey the context alone. Follow the repository's existing convention (conventional commits, etc.).

## Execute

Stage only the relevant files by name. Avoid `git add -A` unless every change belongs in one commit. Create the commit. Show the result.

If pre-commit hooks fail: fix the issue, re-stage, create a NEW commit. Never amend without explicit request — amending after hook failure modifies the previous commit, not the failed one.
