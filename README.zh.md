<p align="center">
  <h1 align="center">cortex-plugin-dev</h1>
  <p align="center"><strong>Cortex 原生开发插件</strong></p>
  <p align="center">
    <a href="https://github.com/by-scott/cortex-plugin-dev/releases"><img src="https://img.shields.io/github/v/release/by-scott/cortex-plugin-dev?display_name=tag" alt="Release"></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License"></a>
  </p>
  <p align="center">
    <a href="README.md">English</a>
  </p>
</p>

---

[Cortex](https://github.com/by-scott/cortex) 官方开发插件。为 Repertoire 层扩展原生编码工具、版本控制集成、基础设施管理和引导式工作流 Skill——基于 `cortex-sdk` 构建，零 Cortex 内部依赖。

## 安装

```bash
cortex plugin install by-scott/cortex-plugin-dev
cortex restart
```

## 工具

### 文件操作、搜索与代码分析

| 工具 | 说明 |
|------|------|
| `read_file` | 带行号和大小保护读取 UTF-8 文件 |
| `write_file` | 通过显式控制创建、覆盖或追加 UTF-8 文件 |
| `replace_in_file` | 带替换次数保护的字面量或正则替换 |
| `glob` | 按模式查找文件（.gitignore 感知，按修改时间排序）|
| `grep` | 正则搜索文件内容（上下文行、匹配计数）|
| `project_map` | 汇总项目语言、入口、包管理器和测试命令 |
| `test_discover` | 不执行命令，仅发现可能的测试和 lint 命令 |
| `dependency_audit` | 检查依赖清单和锁文件，确定审查目标 |
| `secret_scan` | 扫描工作区中可能的凭据和密钥泄漏 |
| `quality_gate` | 从 git、CI、测试、依赖和密钥风险汇总发布就绪状态 |
| `symbols` | 通过 tree-sitter 提取丰富文件符号（Rust、Python、TypeScript、TSX；文档、签名、父级）|
| `symbol_index` | 使用内容哈希 SQLite 缓存索引工作区符号 |
| `symbol_search` | 按符号名、类型、可见性或父级搜索定义 |
| `imports` | 提取导入/依赖关系 |
| `diff` | 比较两个文件（统一 diff 格式，独立于 git）|

### 版本控制

| 工具 | 说明 |
|------|------|
| `git_status` | 工作树状态 |
| `git_diff` | 工作树、暂存区或引用之间的变更 |
| `git_log` | 提交历史 |
| `git_commit` | 暂存文件并创建提交 |
| `worktree_create` | 创建隔离 git worktree 和新分支 |
| `worktree_remove` | 移除 worktree 和分支 |
| `worktree_list` | 列出活跃 worktree |

### 任务管理与规划

| 工具 | 说明 |
|------|------|
| `task_create` | 创建带依赖追踪的结构化任务 |
| `task_list` | 列出任务（状态和阻塞关系）|
| `task_update` | 更新任务状态、元数据、依赖 |
| `enter_plan_mode` | 进入探索/设计阶段 |
| `exit_plan_mode` | 完成规划，准备审查 |
| `todo` | 自由格式会话级笔记 |

### 基础设施与系统

| 工具 | 说明 |
|------|------|
| `diagnostics` | 语言诊断（cargo check、clippy、pyright、mypy、tsc、go vet、eslint）|
| `lsp` | 通过 CLI 和符号索引 fallback 执行语言操作（定义、引用、悬停）|
| `repl` | 执行 Python 或 Node.js 代码 |
| `sql` | 查询 SQLite 数据库（默认只读）|
| `http` | HTTP 请求（GET、POST、PUT、DELETE、PATCH、HEAD）|
| `docker` | Docker 操作（ps、images、run、exec、logs、build、compose）|
| `ps` | 进程列表和端口检查 |
| `notebook_edit` | Jupyter notebook 单元编辑 |
| `env` | 环境变量和系统信息 |

### 通信与协调

| 工具 | 说明 |
|------|------|
| `ask_user` | 带选项的结构化用户提问 |
| `send_message` | 向用户或 Agent 发送消息 |
| `brief` | 对话和任务摘要 |
| `team_create` | 创建 Agent 团队用于并行工作 |
| `team_delete` | 移除团队 |

## Skills

| Skill | 触发词 | 用途 |
|-------|--------|------|
| `commit` | commit、提交 | 审查变更、起草消息、暂存、提交 |
| `review-pr` | review PR、代码审查 | 带严重等级的结构化代码审查 |
| `simplify` | simplify、refactor、简化 | 检测并移除不必要的复杂性 |
| `test` | run test、测试 | 发现、运行、分析失败、修复 |
| `create-pr` | create PR、提交 PR | 起草 PR（摘要和测试计划）|
| `explore` | explore、项目结构 | 映射结构、入口、约定 |
| `debug` | debug、排查 | 复现、隔离、追踪、根因、修复 |
| `implement` | implement、fix、实现 | 定界、编辑、验证并汇报代码变更 |
| `refactor` | refactor、重构 | 保持行为不变的结构改进 |
| `release` | release、publish、发布 | 验证、打包、打 tag 并发布版本化资产 |
| `incident` | incident、logs、故障 | 时间线、证据、复现、根因和修复 |
| `security` | security、secret、安全 | 审查凭据、信任边界和依赖风险 |
| `context-budget` | context、compact、上下文 | 在长任务上下文压力下保留关键状态 |

## 运行时集成

工具通过 `cortex-sdk` 访问 Cortex 运行时：

- **会话感知** — 会话 ID、规范 Actor、来源传输、执行作用域
- **进度发射** — 长操作的逐步更新
- **观察者文本** — 向父 Turn 观察者通道发送诊断信息
- **命名空间状态** — 任务、团队、笔记状态按 Actor/会话隔离
- **后台安全** — diagnostics、process、REPL、Docker 工具声明 `background_safe`

## Tree-sitter 引擎

| 语言 | 扩展名 | 符号类型 |
|------|--------|---------|
| Rust | `.rs` | 函数、方法、trait、impl、宏、结构体、枚举、常量、类型别名、模块、导入 |
| Python | `.py` | 函数、方法、类、常量、导入 |
| TypeScript | `.ts`、`.tsx` | 函数、方法、类、属性、变量、常量、接口、枚举、类型别名、导入 |

## 构建与打包

```bash
cargo build --release
cortex plugin pack .
```

打包器根据 `manifest.toml` 自动从 `target/release/` 定位原生库，并写出 `cortex-plugin-dev-v1.2.0-linux-amd64.cpx`。

## 许可

[MIT](LICENSE)
