use cortex_sdk::{Tool, ToolError, ToolResult};

/// Structured diff between two files or strings — independent of git.
pub struct DiffTool;

impl Tool for DiffTool {
    fn name(&self) -> &'static str {
        "diff"
    }

    fn description(&self) -> &'static str {
        "Compare two files and show differences. Works without git.\n\n\
         Use for comparing any two files, config versions, or before/after states."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_a": { "type": "string", "description": "First file path" },
                "file_b": { "type": "string", "description": "Second file path" },
                "context": { "type": "integer", "description": "Context lines (default: 3)" }
            },
            "required": ["file_a", "file_b"]
        })
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let a = input
            .get("file_a")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("missing 'file_a'".into()))?;
        let b = input
            .get("file_b")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("missing 'file_b'".into()))?;
        let ctx = input
            .get("context")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(3);

        let output = std::process::Command::new("diff")
            .args(["-u", &format!("--context={ctx}"), a, b])
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("diff: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if output.status.success() {
            Ok(ToolResult::success("Files are identical".to_string()))
        } else if output.status.code() == Some(1) {
            Ok(ToolResult::success(stdout.trim().to_string()))
        } else {
            Err(ToolError::ExecutionFailed(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ))
        }
    }
}
