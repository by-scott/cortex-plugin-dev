use cortex_sdk::{Tool, ToolCapabilities, ToolError, ToolResult, ToolRuntime};
use std::sync::{Arc, Mutex};

/// Persistent scratch pad for notes and reminders across turns.
/// Unlike `task_create` (structured work items), todo is freeform text.
pub struct TodoWriteTool {
    notes: Arc<Mutex<std::collections::HashMap<String, Vec<String>>>>,
}

impl TodoWriteTool {
    pub const fn new(notes: Arc<Mutex<std::collections::HashMap<String, Vec<String>>>>) -> Self {
        Self { notes }
    }
}

impl Tool for TodoWriteTool {
    fn name(&self) -> &'static str {
        "todo"
    }

    fn description(&self) -> &'static str {
        "Write a freeform note or reminder. Persists for the session.\n\n\
         Use for quick notes, reminders, scratch calculations, or anything \
         that doesn't fit the structured task system. Call with no content \
         to list existing notes."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "Note content. Empty or omitted to list all notes."
                },
                "clear": {
                    "type": "boolean",
                    "description": "Clear all notes (default: false)"
                }
            }
        })
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        self.execute_in_namespace(&input, "global", None)
    }

    fn execute_with_runtime(
        &self,
        input: serde_json::Value,
        runtime: &dyn ToolRuntime,
    ) -> Result<ToolResult, ToolError> {
        let namespace = super::runtime::namespace(runtime);
        self.execute_in_namespace(&input, &namespace, Some(runtime))
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities {
            emits_observer_text: true,
            ..ToolCapabilities::default()
        }
    }
}

impl TodoWriteTool {
    fn execute_in_namespace(
        &self,
        input: &serde_json::Value,
        namespace: &str,
        runtime: Option<&dyn ToolRuntime>,
    ) -> Result<ToolResult, ToolError> {
        let mut notes = self
            .notes
            .lock()
            .map_err(|e| ToolError::ExecutionFailed(format!("lock: {e}")))?;
        let scoped = notes.entry(namespace.to_string()).or_default();

        if input
            .get("clear")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        {
            let count = scoped.len();
            scoped.clear();
            return Ok(ToolResult::success(format!(
                "Cleared {count} notes in {namespace}"
            )));
        }

        if let Some(content) = input.get("content").and_then(serde_json::Value::as_str)
            && !content.trim().is_empty()
        {
            scoped.push(content.to_string());
            if let Some(runtime) = runtime {
                super::runtime::observe(
                    runtime,
                    "notes",
                    format!("saved note #{} in {namespace}", scoped.len()),
                );
            }
            return Ok(ToolResult::success(format!(
                "Note #{} saved in {namespace}",
                scoped.len()
            )));
        }

        // List notes
        if scoped.is_empty() {
            return Ok(ToolResult::success(format!(
                "No notes in namespace '{namespace}'"
            )));
        }
        let result: Vec<String> = scoped
            .iter()
            .enumerate()
            .map(|(i, n)| format!("{}. {n}", i + 1))
            .collect();
        drop(notes);
        Ok(ToolResult::success(format!(
            "Namespace: {namespace}\n{}",
            result.join("\n")
        )))
    }
}
