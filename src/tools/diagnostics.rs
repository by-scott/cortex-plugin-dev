use cortex_sdk::{Tool, ToolCapabilities, ToolError, ToolResult, ToolRuntime};
use std::process::Command;

/// Run language-specific diagnostics (compile checks, type checks, lint).
/// Uses existing CLI tools — no embedded LSP server needed.
pub struct DiagnosticsTool;

impl Tool for DiagnosticsTool {
    fn name(&self) -> &'static str {
        "diagnostics"
    }

    fn description(&self) -> &'static str {
        "Run compile/type/lint diagnostics on a project or file.\n\n\
         Auto-detects the project type and runs the appropriate checker:\n\
         - Rust: `cargo check` (fast type checking without codegen)\n\
         - Python: `pyright` or `mypy` (type checking)\n\
         - TypeScript: `tsc --noEmit` (type checking)\n\
         - Go: `go vet`\n\n\
         Returns structured diagnostic output (errors and warnings with file:line)."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Project directory or file (default: current directory)"
                },
                "tool": {
                    "type": "string",
                    "description": "Override auto-detection: cargo, pyright, mypy, tsc, go, eslint, clippy",
                    "enum": ["cargo", "clippy", "pyright", "mypy", "tsc", "go", "eslint"]
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
        ToolCapabilities {
            emits_progress: true,
            emits_observer_text: true,
            background_safe: true,
        }
    }
}

impl DiagnosticsTool {
    fn run(
        input: &serde_json::Value,
        runtime: Option<&dyn ToolRuntime>,
    ) -> Result<ToolResult, ToolError> {
        let path = input
            .get("path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(".");
        let tool_override = input.get("tool").and_then(serde_json::Value::as_str);

        if let Some(runtime) = runtime {
            super::runtime::emit_step(runtime, "detecting project diagnostics strategy");
        }

        let tool_name = if let Some(t) = tool_override {
            t.to_string()
        } else {
            detect_project_type(path)?
        };

        if let Some(runtime) = runtime {
            super::runtime::observe(
                runtime,
                "diagnostics",
                format!("running {tool_name} diagnostics in {path}"),
            );
            super::runtime::emit_step(runtime, &format!("executing {tool_name}"));
        }

        let (cmd, args) = match tool_name.as_str() {
            "cargo" => ("cargo", vec!["check", "--message-format=short"]),
            "clippy" => (
                "cargo",
                vec![
                    "clippy",
                    "--message-format=short",
                    "--",
                    "-W",
                    "clippy::pedantic",
                ],
            ),
            "pyright" => ("pyright", vec!["--outputjson"]),
            "mypy" => ("mypy", vec!["."]),
            "tsc" => ("tsc", vec!["--noEmit", "--pretty"]),
            "go" => ("go", vec!["vet", "./..."]),
            "eslint" => ("eslint", vec![".", "--format", "compact"]),
            other => return Err(ToolError::InvalidInput(format!("unknown tool: {other}"))),
        };

        let output = Command::new(cmd)
            .args(&args)
            .current_dir(path)
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("`{cmd}` not found or failed: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let combined = if stdout.is_empty() {
            stderr.to_string()
        } else if stderr.is_empty() {
            stdout.to_string()
        } else {
            format!("{stdout}\n{stderr}")
        };

        if output.status.success() && combined.trim().is_empty() {
            Ok(ToolResult::success(format!(
                "{tool_name}: no diagnostics (clean)"
            )))
        } else if output.status.success() {
            Ok(ToolResult::success(format!(
                "{tool_name} (clean):\n{}",
                combined.trim()
            )))
        } else {
            // Non-zero exit = errors found — return as success (not tool error)
            // so the LLM can read and act on the diagnostics
            Ok(ToolResult::success(format!(
                "{tool_name} found issues:\n{}",
                combined.trim()
            )))
        }
    }
}

fn detect_project_type(path: &str) -> Result<String, ToolError> {
    let p = std::path::Path::new(path);
    if p.join("Cargo.toml").exists() {
        return Ok("cargo".into());
    }
    if p.join("pyproject.toml").exists() || p.join("setup.py").exists() {
        return Ok("pyright".into());
    }
    if p.join("tsconfig.json").exists() {
        return Ok("tsc".into());
    }
    if p.join("go.mod").exists() {
        return Ok("go".into());
    }
    if p.join("package.json").exists() {
        return Ok("eslint".into());
    }
    Err(ToolError::InvalidInput(format!(
        "cannot detect project type in {path}. Specify `tool` explicitly."
    )))
}
