---
description: Prepare, package, and publish a release with versioned artifacts.
when_to_use: Use when the user asks to release, publish, package, tag, or distribute a project.
required_tools:
  - project_map
  - test_discover
  - diagnostics
  - git_status
  - git_diff
  - git_log
tags:
  - release
  - packaging
activation:
  input_patterns:
    - (?i)(release|publish|package|tag|发布|打包|发版)
---

# Release

${ARGS}

## Preflight

Confirm version, release target, artifact names, and repository cleanliness.

## Verify

Run the required test, lint, and packaging checks. Treat warnings as release blockers when the project policy requires it.

## Package

Produce deterministic, versioned artifacts. For Cortex plugins, use `{repository}-v{version}.cpx`.

## Publish

Create a clear commit, tag, release notes, and attach the expected artifacts. Verify installation from the published source.
