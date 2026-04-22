---
description: Run tests, analyze failures, and fix until green
when_to_use: After writing or modifying code that has tests, or when asked to test
required_tools:
  - bash
  - read
  - grep
tags:
  - testing
  - verification
activation:
  input_patterns:
    - (?i)(run test|test.*fail|fix.*test|测试|跑.*测试)
---

# Test

${ARGS}

## Discover

Identify the test framework and run command: `Cargo.toml` → `cargo test`, `package.json` → `npm test` / `jest` / `vitest`, `pytest.ini` / `pyproject.toml` → `pytest`, `Makefile` / `justfile` → look for test targets.

## Run

Execute the full suite (or scoped to the relevant module if the change is isolated). Capture stdout and stderr completely.

## Analyze and Fix

For each failure: read the failing test to understand what it expects, read the implementation it tests. Determine — is the test wrong or is the code wrong? If the code changed recently (`git_diff`), the code is likely at fault.

Apply the minimal fix. One failure at a time. Re-run after each fix to verify no regressions.

## Report

State: how many tests, how many passed, what was fixed. If anything remains broken, explain why and what is needed.
