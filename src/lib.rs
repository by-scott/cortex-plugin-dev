//! Cortex development plugin.
//!
//! Provides file operations, project mapping, code analysis, git integration,
//! task management, diagnostics, HTTP client, Docker, SQL, process management,
//! notebook editing, worktree isolation, and workflow skills.

mod symbol_cache;
mod tools;
pub mod treesitter;

use cortex_sdk::prelude::*;

#[derive(Default)]
struct DevMultiPlugin;

impl MultiToolPlugin for DevMultiPlugin {
    fn plugin_info(&self) -> PluginInfo {
        PluginInfo {
            name: "dev".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            description: "Development tools, infrastructure, and workflow skills".into(),
        }
    }

    fn create_tools(&self) -> Vec<Box<dyn Tool>> {
        let task_store = tools::new_task_store();
        let plan_state = tools::new_plan_state();
        let todo_notes =
            std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
        let team_store = tools::new_team_store();
        vec![
            // File operations and search
            Box::new(tools::ReadFileTool),
            Box::new(tools::WriteFileTool),
            Box::new(tools::ReplaceInFileTool),
            Box::new(tools::GlobTool),
            Box::new(tools::GrepTool),
            // Project understanding
            Box::new(tools::ProjectMapTool),
            Box::new(tools::TestDiscoverTool),
            Box::new(tools::DependencyAuditTool),
            Box::new(tools::SecretScanTool),
            Box::new(tools::QualityGateTool),
            // Code analysis (tree-sitter)
            Box::new(tools::SymbolsTool),
            Box::new(tools::SymbolIndexTool),
            Box::new(tools::SymbolSearchTool),
            Box::new(tools::ImportsTool),
            // Git
            Box::new(tools::GitStatusTool),
            Box::new(tools::GitDiffTool),
            Box::new(tools::GitLogTool),
            Box::new(tools::GitCommitTool),
            // Worktree isolation
            Box::new(tools::WorktreeCreateTool),
            Box::new(tools::WorktreeRemoveTool),
            Box::new(tools::WorktreeListTool),
            // Tasks
            Box::new(tools::TaskCreateTool::new(task_store.clone())),
            Box::new(tools::TaskListTool::new(task_store.clone())),
            Box::new(tools::TaskUpdateTool::new(task_store)),
            // Plan mode
            Box::new(tools::EnterPlanModeTool::new(plan_state.clone())),
            Box::new(tools::ExitPlanModeTool::new(plan_state)),
            // User interaction
            Box::new(tools::AskUserTool),
            Box::new(tools::SendMessageTool),
            // Notes & summary
            Box::new(tools::TodoWriteTool::new(todo_notes)),
            Box::new(tools::BriefTool),
            // Notebook
            Box::new(tools::NotebookEditTool),
            // Infrastructure
            Box::new(tools::DiagnosticsTool),
            Box::new(tools::HttpRequestTool),
            Box::new(tools::DockerTool),
            Box::new(tools::DiffTool),
            Box::new(tools::ProcessTool),
            Box::new(tools::EnvTool),
            Box::new(tools::SqlTool),
            // LSP
            Box::new(tools::LspTool),
            // Team
            Box::new(tools::TeamCreateTool::new(team_store.clone())),
            Box::new(tools::TeamDeleteTool::new(team_store)),
            // REPL
            Box::new(tools::ReplTool),
        ]
    }
}

cortex_sdk::export_plugin!(DevMultiPlugin);
