use cortex_sdk::{Tool, ToolCapabilities, ToolError, ToolResult};
use std::process::Command;

fn run_git(args: &[&str]) -> Result<String, ToolError> {
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|e| ToolError::ExecutionFailed(format!("git: {e}")))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.success() {
        Ok(stdout.trim().to_string())
    } else {
        Err(ToolError::ExecutionFailed(if stderr.is_empty() {
            stdout.to_string()
        } else {
            stderr.trim().to_string()
        }))
    }
}

pub struct WorktreeCreateTool;

impl Tool for WorktreeCreateTool {
    fn name(&self) -> &'static str {
        "worktree_create"
    }

    fn description(&self) -> &'static str {
        "Create an isolated git worktree for safe experimental changes.\n\n\
         Creates a new branch and worktree directory. Changes in the worktree \
         do not affect the main working directory. Use worktree_remove to clean up."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Worktree name (becomes branch name and directory)"
                },
                "base": {
                    "type": "string",
                    "description": "Base ref to branch from (default: HEAD)"
                }
            },
            "required": ["name"]
        })
    }

    fn capabilities(&self) -> ToolCapabilities {
        super::caps([
            super::run_process_effect("git worktree add"),
            super::write_file_effect(".cortex-worktrees/**"),
        ])
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let name = input
            .get("name")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("missing 'name'".into()))?;
        let base = input
            .get("base")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("HEAD");

        // Get repo root
        let root = run_git(&["rev-parse", "--show-toplevel"])?;
        let worktree_dir = format!("{root}/.cortex-worktrees/{name}");

        // Create worktree with new branch
        let branch_name = format!("worktree/{name}");
        run_git(&["worktree", "add", "-b", &branch_name, &worktree_dir, base])?;

        Ok(ToolResult::success(format!(
            "Worktree created:\n  Path: {worktree_dir}\n  Branch: {branch_name}\n  Base: {base}"
        )))
    }
}

pub struct WorktreeRemoveTool;

impl Tool for WorktreeRemoveTool {
    fn name(&self) -> &'static str {
        "worktree_remove"
    }

    fn description(&self) -> &'static str {
        "Remove a git worktree created by worktree_create.\n\n\
         Removes the worktree directory and its branch. Use --force to remove \
         even if there are uncommitted changes."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Worktree name" },
                "force": { "type": "boolean", "description": "Force removal even with uncommitted changes" }
            },
            "required": ["name"]
        })
    }

    fn capabilities(&self) -> ToolCapabilities {
        super::caps([
            super::run_process_effect("git worktree remove"),
            super::delete_file_effect(".cortex-worktrees/**"),
        ])
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let name = input
            .get("name")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("missing 'name'".into()))?;
        let force = input
            .get("force")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        let root = run_git(&["rev-parse", "--show-toplevel"])?;
        let worktree_dir = format!("{root}/.cortex-worktrees/{name}");

        let mut args = vec!["worktree", "remove"];
        if force {
            args.push("--force");
        }
        args.push(&worktree_dir);
        run_git(&args)?;

        // Clean up the branch
        let branch_name = format!("worktree/{name}");
        let _ = run_git(&["branch", "-D", &branch_name]);

        Ok(ToolResult::success(format!("Worktree '{name}' removed")))
    }
}

pub struct WorktreeListTool;

impl Tool for WorktreeListTool {
    fn name(&self) -> &'static str {
        "worktree_list"
    }

    fn description(&self) -> &'static str {
        "List all active git worktrees."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object", "properties": {}})
    }

    fn capabilities(&self) -> ToolCapabilities {
        super::caps([super::run_process_effect("git worktree list")])
    }

    fn execute(&self, _input: serde_json::Value) -> Result<ToolResult, ToolError> {
        run_git(&["worktree", "list"]).map(ToolResult::success)
    }
}
