---
description: Review a pull request or branch diff with structured quality analysis
when_to_use: When reviewing code changes, pull requests, or branch diffs
required_tools:
  - dev
  - read
tags:
  - review
  - quality
activation:
  input_patterns:
    - (?i)(review.*(pr|pull|merge|branch|diff)|pr.*review|代码审查)
---

# Review PR

${ARGS}

## Scope

Identify the changes: `git_diff` against the base branch. Understand size and nature. Read the PR description or commit messages for stated intent. If multiple commits, review ALL of them — the full scope, not just the latest.

## Examine

For each changed file: does the change match stated intent? Are there unrelated modifications mixed in? Is the change complete, or are TODO/FIXME markers left behind?

Probe correctness: logic errors, off-by-one, null/undefined handling, edge cases without test coverage, concurrency issues if applicable. Check security: input validation, injection vectors, credential exposure.

Assess quality: naming clarity, abstraction level, separation of concerns. Are failures surfaced or swallowed? Unnecessary allocations, N+1 queries, missing indexes? Does the change follow existing codebase conventions?

## Report

Per finding: **File:Line** | **Severity** (critical / warning / nit) | **Issue** | **Suggestion**

Summary verdict: approve / request changes / block. State what was done well, not only what needs work.
