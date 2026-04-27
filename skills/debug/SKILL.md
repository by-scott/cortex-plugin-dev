---
description: Interactive debugging — reproduce, isolate, trace, fix
when_to_use: When something doesn't work and the cause isn't obvious from reading code
required_tools:
  - bash
  - read
  - grep
tags:
  - debugging
  - troubleshooting
activation:
  input_patterns:
    - (?i)(debug|doesn.*work|not working|怎么.*不|排查|调试)
---

# Debug

${ARGS}

## Reproduce

Run the failing scenario. Capture the exact error: message, stack trace, exit code, log output. If it cannot be reproduced, clarify the conditions under which it fails.

## Isolate

Narrow the scope. Does it fail with minimal input? Which component produces the first incorrect output? What changed between "it worked" and "it broke"? (`git_log`, `git_diff`)

## Trace

Follow the actual execution path, not the assumed one. Add targeted logging at decision points. Read the code the runtime actually executes — follow imports, not mental models. Check configuration, environment variables, file permissions, network state.

## Root Cause

Name the exact line or condition. Explain the mechanism: not "it's broken here" but "this value is null because the initialization path skips it when X is true."

## Fix

Change only what the root cause requires. Run the original failing scenario to confirm. Run the full test suite for regressions. Search for the same pattern elsewhere in the codebase.
