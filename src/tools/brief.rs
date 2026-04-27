use cortex_sdk::{Tool, ToolCapabilities, ToolError, ToolResult};

/// Summarize the current conversation context for the user.
pub struct BriefTool;

impl Tool for BriefTool {
    fn name(&self) -> &'static str {
        "brief"
    }

    fn description(&self) -> &'static str {
        "Generate a brief summary of the current conversation and task state.\n\n\
         Use at natural checkpoints: after completing a phase of work, \
         before switching topics, or when the user asks 'where are we?'"
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "focus": {
                    "type": "string",
                    "description": "What aspect to focus on (e.g., 'decisions', 'remaining work', 'changes made')"
                }
            }
        })
    }

    fn capabilities(&self) -> ToolCapabilities {
        super::caps([super::introspect_effect("conversation state")])
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let focus = input
            .get("focus")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("overall");
        // The tool returns a prompt for the LLM to generate the summary.
        // The LLM has the full conversation context; this tool signals intent.
        Ok(ToolResult::success(format!(
            "Generate a brief summary focusing on: {focus}\n\
             Include: key decisions made, current task state, what remains to be done.\n\
             Be concise — this is a checkpoint, not a report."
        )))
    }
}
