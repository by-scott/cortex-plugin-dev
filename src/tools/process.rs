use cortex_sdk::{Tool, ToolCapabilities, ToolError, ToolResult, ToolRuntime};
use std::process::Command;

fn parse_limit(input: &serde_json::Value) -> Result<usize, ToolError> {
    input
        .get("limit")
        .and_then(serde_json::Value::as_u64)
        .map_or(Ok(20), |value| {
            usize::try_from(value).map_err(|_| {
                ToolError::InvalidInput(format!("'limit' is too large for this platform: {value}"))
            })
        })
}

/// List, find, and manage system processes.
pub struct ProcessTool;

impl Tool for ProcessTool {
    fn name(&self) -> &'static str {
        "ps"
    }

    fn description(&self) -> &'static str {
        "List or find system processes.\n\n\
         Sub-commands:\n\
         - `list`: Show running processes (top by CPU)\n\
         - `find`: Find processes by name\n\
         - `ports`: Show listening ports and their processes"
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "enum": ["list", "find", "ports"],
                    "description": "Sub-command (default: list)"
                },
                "query": {
                    "type": "string",
                    "description": "Process name filter (for 'find')"
                },
                "limit": {
                    "type": "integer",
                    "description": "Max results (default: 20)"
                }
            }
        })
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        Self::run(&input, None)
    }

    fn execute_with_runtime(
        &self,
        input: serde_json::Value,
        runtime: &dyn ToolRuntime,
    ) -> Result<ToolResult, ToolError> {
        Self::run(&input, Some(runtime))
    }

    fn capabilities(&self) -> ToolCapabilities {
        super::progress_caps([super::run_process_effect("process inspection")], true)
    }
}

impl ProcessTool {
    fn run(
        input: &serde_json::Value,
        runtime: Option<&dyn ToolRuntime>,
    ) -> Result<ToolResult, ToolError> {
        let cmd = input
            .get("command")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("list");
        let limit = parse_limit(input)?;

        if let Some(runtime) = runtime {
            super::runtime::emit_step(runtime, &format!("collecting process data via '{cmd}'"));
        }

        match cmd {
            "list" => {
                let output = Command::new("ps")
                    .args(["aux", "--sort=-%cpu"])
                    .output()
                    .map_err(|e| ToolError::ExecutionFailed(format!("ps: {e}")))?;
                let stdout = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<&str> = stdout.lines().take(limit + 1).collect();
                Ok(ToolResult::success(lines.join("\n")))
            }
            "find" => {
                let query = input
                    .get("query")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| ToolError::InvalidInput("'query' required for find".into()))?;
                let output = Command::new("pgrep")
                    .args(["-a", query])
                    .output()
                    .map_err(|e| ToolError::ExecutionFailed(format!("pgrep: {e}")))?;
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.trim().is_empty() {
                    Ok(ToolResult::success(format!(
                        "No processes matching '{query}'"
                    )))
                } else {
                    Ok(ToolResult::success(stdout.trim().to_string()))
                }
            }
            "ports" => {
                let output = Command::new("ss")
                    .args(["-tlnp"])
                    .output()
                    .map_err(|e| ToolError::ExecutionFailed(format!("ss: {e}")))?;
                Ok(ToolResult::success(
                    String::from_utf8_lossy(&output.stdout).trim().to_string(),
                ))
            }
            other => Err(ToolError::InvalidInput(format!("unknown command: {other}"))),
        }
    }
}
