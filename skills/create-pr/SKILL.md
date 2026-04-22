---
description: Create a pull request with structured summary and test plan
when_to_use: When work is complete and ready to submit as a PR
required_tools:
  - bash
  - git_status
  - git_diff
  - git_log
tags:
  - git
  - workflow
activation:
  input_patterns:
    - (?i)(create.*pr|pull.*request|open.*pr|提交.*pr|创建.*pr)
---

# Create PR

${ARGS}

## Survey

Run `git_status` and `git_diff` to understand all changes. Run `git_log` to see commit history since diverging from the base branch. Verify nothing is uncommitted that should be included.

## Analyze

Review ALL commits that will be included — not just the latest. Understand the full scope. Classify: feature, fix, refactor, docs, test.

## Draft

**Title**: short, under 70 characters, imperative voice.

**Body**:
```
## Summary
- 1-3 bullet points explaining what and why

## Test plan
- How to verify the changes work
- What was tested
- What automated tests cover
```

## Execute

Create branch if needed. Push to remote with `-u`. Create the PR using `gh pr create` (if available) or provide manual steps. Return the PR URL.
