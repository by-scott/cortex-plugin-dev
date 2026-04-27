use cortex_sdk::{Tool, ToolCapabilities, ToolError, ToolResult, ToolRuntime};
use std::process::Command;

/// Execute code in a language REPL (Python or Node.js).
pub struct ReplTool;

impl Tool for ReplTool {
    fn name(&self) -> &'static str {
        "repl"
    }

    fn description(&self) -> &'static str {
        "Execute a code snippet in Python or Node.js.\n\n\
         Use for quick calculations, data processing, or testing code fragments \
         without creating a file. Each invocation is stateless."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "lang": {
                    "type": "string",
                    "enum": ["python", "node"],
                    "description": "Language runtime (default: python)"
                },
                "code": {
                    "type": "string",
                    "description": "Code to execute"
                }
            },
            "required": ["code"]
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
        super::progress_caps([super::run_process_effect("language REPL")], true)
    }
}

impl ReplTool {
    fn run(
        input: &serde_json::Value,
        runtime: Option<&dyn ToolRuntime>,
    ) -> Result<ToolResult, ToolError> {
        let code = input
            .get("code")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("missing 'code'".into()))?;
        let lang = input
            .get("lang")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("python");

        let (cmd, args): (&str, Vec<String>) = match lang {
            "python" => ("python3", vec!["-c".into(), code.into()]),
            "node" => ("node", vec!["-e".into(), code.into()]),
            other => return Err(ToolError::InvalidInput(format!("unsupported: {other}"))),
        };

        if let Some(runtime) = runtime {
            super::runtime::observe(runtime, "repl", format!("executing {lang} snippet"));
            super::runtime::emit_step(runtime, &format!("running {cmd}"));
        }

        let output = Command::new(cmd)
            .args(&args)
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("`{cmd}` not found: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            let result = if stdout.is_empty() {
                "(no output)".to_string()
            } else {
                stdout.trim().to_string()
            };
            Ok(ToolResult::success(result))
        } else {
            Ok(ToolResult::error(
                format!("{}\n{}", stdout.trim(), stderr.trim())
                    .trim()
                    .to_string(),
            ))
        }
    }
}
