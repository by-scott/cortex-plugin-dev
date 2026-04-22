use cortex_sdk::{Tool, ToolError, ToolResult};

/// Read environment variables and system info.
pub struct EnvTool;

impl Tool for EnvTool {
    fn name(&self) -> &'static str {
        "env"
    }

    fn description(&self) -> &'static str {
        "Read environment variables and system information.\n\n\
         Use to check PATH, HOME, LANG, shell, or any env var. \
         Also reports OS, architecture, and working directory."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "var": {
                    "type": "string",
                    "description": "Specific variable name (omit to show system summary)"
                }
            }
        })
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        if let Some(var) = input.get("var").and_then(serde_json::Value::as_str) {
            let val = std::env::var(var).unwrap_or_else(|_| "(not set)".into());
            return Ok(ToolResult::success(format!("{var}={val}")));
        }

        // System summary
        let mut info = Vec::new();
        info.push(format!("OS: {}", std::env::consts::OS));
        info.push(format!("Arch: {}", std::env::consts::ARCH));
        if let Ok(cwd) = std::env::current_dir() {
            info.push(format!("CWD: {}", cwd.display()));
        }
        for key in ["HOME", "USER", "SHELL", "LANG", "PATH", "EDITOR"] {
            let val = std::env::var(key).unwrap_or_else(|_| "(not set)".into());
            let display = if key == "PATH" && val.len() > 100 {
                format!("{}...", &val[..100])
            } else {
                val
            };
            info.push(format!("{key}: {display}"));
        }

        Ok(ToolResult::success(info.join("\n")))
    }
}
