---
description: Review security-sensitive changes and local secret exposure before commit or release.
when_to_use: Use when code touches auth, input handling, secrets, network boundaries, data storage, or release packaging.
required_tools:
  - secret_scan
  - dependency_audit
  - grep
  - read_file
  - diagnostics
tags:
  - security
  - review
activation:
  input_patterns:
    - (?i)(security|secret|auth|token|安全|密钥|权限|认证)
---

# Security

${ARGS}

## Boundaries

Identify trust boundaries: user input, filesystem, network, database, provider calls, credentials, and channel delivery.

## Scan

Run local secret scanning and inspect dependency manifests. Treat any hardcoded credential as a release blocker.

## Review

Check validation, authorization, error messages, logging, and unsafe shell/process use. Prefer concrete exploit paths over generic warnings.

## Close

Report findings by severity, include file locations, and state what was verified.
