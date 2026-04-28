<p align="center">
  <h1 align="center">cortex-plugin-dev</h1>
  <p align="center"><strong>Native development plugin for Cortex</strong></p>
  <p align="center">
    <a href="https://github.com/by-scott/cortex-plugin-dev/releases"><img src="https://img.shields.io/github/v/release/by-scott/cortex-plugin-dev?display_name=tag" alt="Release"></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License"></a>
  </p>
  <p align="center">
    <a href="README.zh.md">中文</a>
  </p>
</p>

---

The official development plugin for [Cortex](https://github.com/by-scott/cortex). Extends the Repertoire layer with native coding tools, version control integration, infrastructure management, and guided workflow skills — all built on `cortex-sdk` with zero dependency on Cortex internals.

## Install

```bash
cortex plugin install by-scott/cortex-plugin-dev --yes
cortex restart
```

## Tools

### File Operations, Search & Code Analysis

| Tool | Description |
|------|-------------|
| `read_file` | Read UTF-8 files with line numbers and size guards |
| `write_file` | Create, overwrite, or append UTF-8 files with explicit controls |
| `replace_in_file` | Literal or regex replacement with replacement-count guards |
| `glob` | Find files by pattern (.gitignore-aware, sorted by modification time) |
| `grep` | Search file contents with regex (context lines, match counts) |
| `project_map` | Summarize project languages, entry points, package managers, and test commands |
| `test_discover` | Discover likely test and lint commands without running them |
| `dependency_audit` | Inspect dependency manifests and lockfiles for review targets |
| `secret_scan` | Scan workspace files for likely credentials and secret leaks |
| `quality_gate` | Summarize release readiness from git, CI, tests, dependencies, and secrets |
| `symbols` | Extract rich file symbols via tree-sitter (Rust, Python, TypeScript, TSX; docs, signatures, parents) |
| `symbol_index` | Index workspace symbols with a content-hash SQLite cache |
| `symbol_search` | Search definitions by symbol name, kind, visibility, or parent |
| `imports` | Extract import/dependency relationships |
| `diff` | Compare two files (unified diff, independent of git) |

### Version Control

| Tool | Description |
|------|-------------|
| `git_status` | Working tree status |
| `git_diff` | Changes between working tree, staging, or references |
| `git_log` | Commit history |
| `git_commit` | Stage files and create commit |
| `worktree_create` | Create isolated git worktree with new branch |
| `worktree_remove` | Remove worktree and branch |
| `worktree_list` | List active worktrees |

### Task Management & Planning

| Tool | Description |
|------|-------------|
| `task_create` | Create structured task with dependency tracking |
| `task_list` | List tasks with status and blockers |
| `task_update` | Update task status, metadata, dependencies |
| `enter_plan_mode` | Signal exploration/design phase |
| `exit_plan_mode` | Complete planning, ready for review |
| `todo` | Freeform session-scoped notes |

### Infrastructure & System

| Tool | Description |
|------|-------------|
| `diagnostics` | Language diagnostics (cargo check, clippy, pyright, mypy, tsc, go vet, eslint) |
| `lsp` | Language operations via CLI and symbol-index fallback (definition, references, hover) |
| `repl` | Execute Python or Node.js code |
| `sql` | Query SQLite databases (read-only by default) |
| `http` | HTTP requests (GET, POST, PUT, DELETE, PATCH, HEAD) |
| `docker` | Docker operations (ps, images, run, exec, logs, build, compose) |
| `ps` | Process listing and port inspection |
| `notebook_edit` | Jupyter notebook cell editing |
| `env` | Environment variables and system info |

### Communication & Coordination

| Tool | Description |
|------|-------------|
| `ask_user` | Structured prompts with options |
| `send_message` | Message to user or agent |
| `brief` | Conversation and task summarization |
| `team_create` | Create agent team for parallel work |
| `team_delete` | Remove team |

## Skills

| Skill | Trigger | Purpose |
|-------|---------|---------|
| `commit` | commit, 提交 | Review changes, draft message, stage, commit |
| `review-pr` | review PR, 代码审查 | Structured code review with severity-rated findings |
| `simplify` | simplify, refactor, 简化 | Detect and remove unnecessary complexity |
| `test` | run test, 测试 | Discover, run, analyze failures, fix |
| `create-pr` | create PR, 提交 PR | Draft PR with summary and test plan |
| `explore` | explore, 项目结构 | Map structure, entry points, conventions |
| `debug` | debug, 排查 | Reproduce, isolate, trace, root-cause, fix |
| `implement` | implement, fix, 实现 | Scope, edit, verify, and report code changes |
| `refactor` | refactor, 重构 | Preserve behavior while improving structure |
| `release` | release, publish, 发布 | Verify, package, tag, and publish versioned artifacts |
| `incident` | incident, logs, 故障 | Timeline, evidence, reproduction, root cause, fix |
| `security` | security, secret, 安全 | Review credentials, trust boundaries, and dependency risk |
| `context-budget` | context, compact, 上下文 | Preserve critical state under long-task context pressure |

## Runtime Integration

Tools access Cortex runtime via `cortex-sdk`:

- **Session awareness** — session ID, canonical actor, source transport, execution scope
- **Progress emission** — step-by-step updates for long operations
- **Observer text** — diagnostic information to the parent turn's observer lane
- **Namespaced state** — task, team, and note state isolated per actor/session
- **Background safety** — diagnostics, process, REPL, Docker tools declare `background_safe`

## Tree-sitter Engine

| Language | Extensions | Symbol Types |
|----------|-----------|-------------|
| Rust | `.rs` | functions, methods, traits, impls, macros, structs, enums, constants, type aliases, modules, imports |
| Python | `.py` | functions, methods, classes, constants, imports |
| TypeScript | `.ts`, `.tsx` | functions, methods, classes, properties, variables, constants, interfaces, enums, type aliases, imports |

## Build & Package

```bash
cargo build --release
cortex plugin sign . --key /path/to/publisher.ed25519 --publisher by-scott
cortex plugin pack .
```

The packer auto-resolves the native library from `target/release/` based on `manifest.toml` and writes `cortex-plugin-dev-v1.5.7-linux-amd64.cpx`. Packaged installs require a valid Cortex package signature; `--yes` should only be used after the publisher key fingerprint has been reviewed.

## License

[MIT](LICENSE)
