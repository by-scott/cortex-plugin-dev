use cortex_sdk::{Tool, ToolCapabilities, ToolError, ToolResult, ToolRuntime};
use std::process::Command;

fn run_docker(args: &[&str]) -> Result<String, ToolError> {
    let output = Command::new("docker")
        .args(args)
        .output()
        .map_err(|e| ToolError::ExecutionFailed(format!("docker: {e}")))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.success() {
        Ok(stdout.trim().to_string())
    } else {
        Err(ToolError::ExecutionFailed(stderr.trim().to_string()))
    }
}

pub struct DockerTool;

impl Tool for DockerTool {
    fn name(&self) -> &'static str {
        "docker"
    }

    fn description(&self) -> &'static str {
        "Run Docker commands for container management.\n\n\
         Sub-commands:\n\
         - `ps`: List running containers\n\
         - `images`: List images\n\
         - `run`: Run a command in a new container\n\
         - `exec`: Execute in a running container\n\
         - `logs`: View container logs\n\
         - `build`: Build an image from Dockerfile\n\
         - `compose`: Run docker compose commands"
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Docker sub-command: ps, images, run, exec, logs, build, compose",
                    "enum": ["ps", "images", "run", "exec", "logs", "build", "compose"]
                },
                "args": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Additional arguments"
                }
            },
            "required": ["command"]
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
        super::progress_caps([super::run_process_effect("docker")], false)
    }
}

impl DockerTool {
    fn run(
        input: &serde_json::Value,
        runtime: Option<&dyn ToolRuntime>,
    ) -> Result<ToolResult, ToolError> {
        let command = input
            .get("command")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("missing 'command'".into()))?;

        let extra_args: Vec<String> = input
            .get("args")
            .and_then(serde_json::Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let mut args: Vec<&str> = Vec::new();

        match command {
            "compose" => {
                // docker compose <args>
                args.push("compose");
                for a in &extra_args {
                    args.push(a);
                }
            }
            other => {
                args.push(other);
                for a in &extra_args {
                    args.push(a);
                }
            }
        }

        if let Some(runtime) = runtime {
            super::runtime::observe(
                runtime,
                "docker",
                format!("running docker {command} {}", extra_args.join(" ")),
            );
            super::runtime::emit_step(runtime, &format!("invoking docker {command}"));
        }

        run_docker(&args).map(ToolResult::success)
    }
}
