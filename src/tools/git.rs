use cortex_sdk::{Tool, ToolCapabilities, ToolError, ToolResult};
use std::process::Command;

fn run_git(args: &[&str]) -> Result<String, ToolError> {
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|e| ToolError::ExecutionFailed(format!("failed to run git: {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        Ok(stdout.trim().to_string())
    } else {
        Err(ToolError::ExecutionFailed(format!(
            "git {} failed: {}",
            args.join(" "),
            if stderr.is_empty() {
                stdout.to_string()
            } else {
                stderr.to_string()
            }
        )))
    }
}

// ── git_status ──────────────────────────────────────────────

pub struct GitStatusTool;

impl Tool for GitStatusTool {
    fn name(&self) -> &'static str {
        "git_status"
    }

    fn description(&self) -> &'static str {
        "Show git working tree status. Structured alternative to `bash` with `git status`."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "short": {
                    "type": "boolean",
                    "description": "Short format output (default: true)"
                }
            }
        })
    }

    fn capabilities(&self) -> ToolCapabilities {
        super::caps([super::run_process_effect("git status")])
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let short = input
            .get("short")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);

        let mut args = vec!["status"];
        if short {
            args.push("--short");
            args.push("--branch");
        }

        run_git(&args).map(ToolResult::success)
    }
}

// ── git_diff ────────────────────────────────────────────────

pub struct GitDiffTool;

impl Tool for GitDiffTool {
    fn name(&self) -> &'static str {
        "git_diff"
    }

    fn description(&self) -> &'static str {
        "Show changes in the working tree, staging area, or between commits."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "staged": {
                    "type": "boolean",
                    "description": "Show staged changes (--cached)"
                },
                "ref": {
                    "type": "string",
                    "description": "Compare against a ref (e.g., HEAD~3, main)"
                },
                "path": {
                    "type": "string",
                    "description": "Limit diff to a specific path"
                },
                "stat": {
                    "type": "boolean",
                    "description": "Show diffstat only (default: false)"
                }
            }
        })
    }

    fn capabilities(&self) -> ToolCapabilities {
        super::caps([
            super::run_process_effect("git diff"),
            super::read_file_effect("repository paths"),
        ])
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let mut args = vec!["diff"];

        let staged = input
            .get("staged")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        if staged {
            args.push("--cached");
        }

        let ref_val = input.get("ref").and_then(serde_json::Value::as_str);
        if let Some(r) = ref_val {
            args.push(r);
        }

        let stat = input
            .get("stat")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        if stat {
            args.push("--stat");
        }

        let path_val = input.get("path").and_then(serde_json::Value::as_str);
        if let Some(p) = path_val {
            args.push("--");
            args.push(p);
        }

        let result = run_git(&args)?;
        if result.is_empty() {
            Ok(ToolResult::success("No changes".to_string()))
        } else {
            Ok(ToolResult::success(result))
        }
    }
}

// ── git_log ─────────────────────────────────────────────────

pub struct GitLogTool;

impl Tool for GitLogTool {
    fn name(&self) -> &'static str {
        "git_log"
    }

    fn description(&self) -> &'static str {
        "Show recent commit history."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "count": {
                    "type": "integer",
                    "description": "Number of commits to show (default: 10)"
                },
                "oneline": {
                    "type": "boolean",
                    "description": "One-line format (default: true)"
                },
                "path": {
                    "type": "string",
                    "description": "Show history for specific path"
                }
            }
        })
    }

    fn capabilities(&self) -> ToolCapabilities {
        super::caps([super::run_process_effect("git log")])
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let count = input
            .get("count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(10);
        let count_str = format!("-{count}");

        let oneline = input
            .get("oneline")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);

        let mut args = vec!["log", &count_str];
        if oneline {
            args.push("--oneline");
        }

        let path_val = input.get("path").and_then(serde_json::Value::as_str);
        if let Some(p) = path_val {
            args.push("--");
            args.push(p);
        }

        run_git(&args).map(ToolResult::success)
    }
}

// ── git_commit ──────────────────────────────────────────────

pub struct GitCommitTool;

impl Tool for GitCommitTool {
    fn name(&self) -> &'static str {
        "git_commit"
    }

    fn description(&self) -> &'static str {
        "Stage files and create a git commit.\n\n\
         Use the /commit skill for guided commit workflow.\n\
         This tool is for direct, precise commits when you know exactly what to stage."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "Commit message"
                },
                "files": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Files to stage (default: all modified/new tracked files)"
                },
                "all": {
                    "type": "boolean",
                    "description": "Stage all changes including untracked (default: false)"
                }
            },
            "required": ["message"]
        })
    }

    fn capabilities(&self) -> ToolCapabilities {
        super::caps([
            super::run_process_effect("git add/commit"),
            super::write_file_effect(".git/**"),
        ])
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let message = input
            .get("message")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("missing 'message'".into()))?;

        let all = input
            .get("all")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        let files = input
            .get("files")
            .and_then(serde_json::Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(serde_json::Value::as_str)
                    .collect::<Vec<_>>()
            });

        // Stage
        if let Some(ref file_list) = files {
            let mut add_args: Vec<&str> = vec!["add"];
            add_args.extend(file_list.iter());
            run_git(&add_args)?;
        } else if all {
            run_git(&["add", "-A"])?;
        }

        // Commit
        let result = run_git(&["commit", "-m", message])?;
        Ok(ToolResult::success(result))
    }
}
