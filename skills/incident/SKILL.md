---
description: Investigate production-like failures from symptoms, logs, and reproduction steps.
when_to_use: Use when behavior is intermittent, channel-specific, timing-sensitive, or observed in logs.
required_tools:
  - read_file
  - grep
  - project_map
  - diagnostics
  - ps
tags:
  - debug
  - incident
activation:
  input_patterns:
    - (?i)(incident|outage|logs|timeout|丢消息|超时|日志|故障)
---

# Incident

${ARGS}

## Timeline

Establish exact user-visible symptoms, timestamps, affected clients, and expected behavior.

## Evidence

Read logs and persisted session state before changing code. Separate transport failure, runtime failure, provider failure, and rendering failure.

## Reproduce

Create the smallest repeatable scenario. If intermittent, identify timing windows and retry boundaries.

## Fix

Patch the owning layer, not the symptom. Add regression coverage or an operational check where possible.

## Close

Report root cause, fix, verification, and any remaining monitoring gap.
