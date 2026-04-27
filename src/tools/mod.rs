mod ask;
mod brief;
mod diagnostics;
mod diff;
mod docker;
mod env;
mod file_ops;
mod git;
mod glob;
mod grep;
mod http_client;
mod lsp;
mod notebook;
mod plan;
mod process;
mod project;
mod quality;
mod repl;
mod runtime;
mod send_message;
mod sql;
mod symbols;
mod task;
mod team;
mod todo;
mod worktree;

use cortex_sdk::{
    DryRunSupport, EffectConfirmation, EffectReversibility, ToolCapabilities, ToolEffect,
    ToolEffectKind,
};

pub(crate) fn caps(effects: impl IntoIterator<Item = ToolEffect>) -> ToolCapabilities {
    ToolCapabilities {
        effects: effects.into_iter().collect(),
        ..ToolCapabilities::default()
    }
}

pub(crate) fn progress_caps(
    effects: impl IntoIterator<Item = ToolEffect>,
    background_safe: bool,
) -> ToolCapabilities {
    ToolCapabilities {
        emits_progress: true,
        emits_observer_text: true,
        background_safe,
        effects: effects.into_iter().collect(),
    }
}

pub(crate) fn observer_caps(effects: impl IntoIterator<Item = ToolEffect>) -> ToolCapabilities {
    ToolCapabilities {
        emits_observer_text: true,
        effects: effects.into_iter().collect(),
        ..ToolCapabilities::default()
    }
}

pub(crate) fn read_file_effect(target: impl Into<String>) -> ToolEffect {
    ToolEffect::new(ToolEffectKind::ReadFile)
        .with_target(target)
        .with_confirmation(EffectConfirmation::OnRisk)
}

pub(crate) fn write_file_effect(target: impl Into<String>) -> ToolEffect {
    ToolEffect::new(ToolEffectKind::WriteFile)
        .with_target(target)
        .with_confirmation(EffectConfirmation::Always)
        .with_reversibility(EffectReversibility::PartiallyReversible)
        .with_dry_run(DryRunSupport::Supported)
}

pub(crate) fn delete_file_effect(target: impl Into<String>) -> ToolEffect {
    ToolEffect::new(ToolEffectKind::DeleteFile)
        .with_target(target)
        .with_confirmation(EffectConfirmation::Always)
        .with_reversibility(EffectReversibility::Irreversible)
}

pub(crate) fn run_process_effect(target: impl Into<String>) -> ToolEffect {
    ToolEffect::new(ToolEffectKind::RunProcess)
        .with_target(target)
        .with_confirmation(EffectConfirmation::Always)
}

pub(crate) fn network_effect(target: impl Into<String>) -> ToolEffect {
    ToolEffect::new(ToolEffectKind::NetworkRequest)
        .with_target(target)
        .with_confirmation(EffectConfirmation::OnRisk)
}

pub(crate) fn send_message_effect(target: impl Into<String>) -> ToolEffect {
    ToolEffect::new(ToolEffectKind::SendMessage)
        .with_target(target)
        .with_confirmation(EffectConfirmation::Always)
}

pub(crate) fn schedule_effect(target: impl Into<String>) -> ToolEffect {
    ToolEffect::new(ToolEffectKind::ScheduleTask)
        .with_target(target)
        .with_confirmation(EffectConfirmation::Always)
}

pub(crate) fn introspect_effect(target: impl Into<String>) -> ToolEffect {
    ToolEffect::new(ToolEffectKind::IntrospectRuntime)
        .with_target(target)
        .with_confirmation(EffectConfirmation::OnRisk)
}

pub(crate) fn delegate_effect(target: impl Into<String>) -> ToolEffect {
    ToolEffect::new(ToolEffectKind::DelegateWork)
        .with_target(target)
        .with_confirmation(EffectConfirmation::Always)
}

pub use ask::AskUserTool;
pub use brief::BriefTool;
pub use diagnostics::DiagnosticsTool;
pub use diff::DiffTool;
pub use docker::DockerTool;
pub use env::EnvTool;
pub use file_ops::{ReadFileTool, ReplaceInFileTool, WriteFileTool};
pub use git::{GitCommitTool, GitDiffTool, GitLogTool, GitStatusTool};
pub use glob::GlobTool;
pub use grep::GrepTool;
pub use http_client::HttpRequestTool;
pub use lsp::LspTool;
pub use notebook::NotebookEditTool;
pub use plan::{EnterPlanModeTool, ExitPlanModeTool, new_plan_state};
pub use process::ProcessTool;
pub use project::{DependencyAuditTool, ProjectMapTool, TestDiscoverTool};
pub use quality::{QualityGateTool, SecretScanTool};
pub use repl::ReplTool;
pub use send_message::SendMessageTool;
pub use sql::SqlTool;
pub use symbols::{ImportsTool, SymbolIndexTool, SymbolSearchTool, SymbolsTool};
pub use task::{TaskCreateTool, TaskListTool, TaskUpdateTool, new_task_store};
pub use team::{TeamCreateTool, TeamDeleteTool, new_team_store};
pub use todo::TodoWriteTool;
pub use worktree::{WorktreeCreateTool, WorktreeListTool, WorktreeRemoveTool};
