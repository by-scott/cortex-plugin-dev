use cortex_sdk::{Tool, ToolCapabilities, ToolError, ToolResult};

/// Send a message to the user or to another agent.
/// For user messages: ensures output is visible (not buried in tool results).
/// For agent messages: coordination signal in multi-agent scenarios.
pub struct SendMessageTool;

impl Tool for SendMessageTool {
    fn name(&self) -> &'static str {
        "send_message"
    }

    fn description(&self) -> &'static str {
        "Send a visible message to the user or to a named agent.\n\n\
         Use when you need to ensure a message is prominently visible — \
         not buried in tool output. Also used for agent-to-agent coordination \
         when working in parallel."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "to": {
                    "type": "string",
                    "description": "Recipient: 'user' or agent name (default: 'user')"
                },
                "message": {
                    "type": "string",
                    "description": "The message content"
                }
            },
            "required": ["message"]
        })
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let message = input
            .get("message")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("missing 'message'".into()))?;
        let to = input
            .get("to")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("user");

        // The message content becomes the tool result, which is included
        // in the response to the user or forwarded to the target agent.
        Ok(ToolResult::success(format!("[To {to}]: {message}")))
    }

    fn capabilities(&self) -> ToolCapabilities {
        super::caps([super::send_message_effect("recipient")])
    }
}
