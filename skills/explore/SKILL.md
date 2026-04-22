---
description: Explore a codebase or directory to understand its structure and purpose
when_to_use: When entering an unfamiliar project, directory, or module
required_tools:
  - glob
  - grep
  - read
tags:
  - exploration
  - understanding
activation:
  input_patterns:
    - (?i)(explore|what.*is.*this|show.*structure|项目结构|看看)
---

# Explore

${ARGS}

## Shape

Use `glob` to map top-level structure. Count files by type. Identify the organizational pattern: flat, layered, feature-based, monorepo.

## Entry Points

Find and read: manifest files (Cargo.toml, package.json, pyproject.toml, go.mod), README, CONTRIBUTING, architecture docs, main entry point. Map the dependency graph: what depends on what, are there layers, are there circular references.

## Conventions

Use `grep` to find recurring patterns: error handling style, testing approach, configuration management, naming conventions. Identifying conventions early accelerates all subsequent work.

## Report

Purpose (one sentence) · Stack (language, framework, dependencies) · Structure (directory layout with role of each unit) · Entry points · Conventions · Key files (3-5 highest information-density files).
