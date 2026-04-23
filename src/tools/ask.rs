use cortex_sdk::{Tool, ToolError, ToolResult};
use std::fmt::Write;

/// A tool that signals the LLM should ask the user a structured question.
///
/// The actual user interaction is handled by the runtime's permission/UI layer.
/// This tool returns a formatted question prompt that the LLM should present.
pub struct AskUserTool;

impl Tool for AskUserTool {
    fn name(&self) -> &'static str {
        "ask_user"
    }

    fn description(&self) -> &'static str {
        "Ask the user a question to clarify requirements or get a decision.\n\n\
         Use when you need user input before proceeding — don't guess.\n\
         Formulate a clear question with specific options when possible.\n\
         The question is returned as the tool result for you to present."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "question": {
                    "type": "string",
                    "description": "The question to ask the user"
                },
                "options": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "label": { "type": "string" },
                            "description": { "type": "string" }
                        },
                        "required": ["label"]
                    },
                    "description": "Optional: specific choices for the user"
                },
                "context": {
                    "type": "string",
                    "description": "Why you're asking this question"
                }
            },
            "required": ["question"]
        })
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let question = input
            .get("question")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("missing 'question'".into()))?;

        let mut output = String::new();

        if let Some(ctx) = input.get("context").and_then(serde_json::Value::as_str) {
            let _ = write!(output, "Context: {ctx}\n\n");
        }

        let _ = write!(output, "Question: {question}");

        if let Some(options) = input.get("options").and_then(serde_json::Value::as_array) {
            output.push_str("\n\nOptions:");
            for (i, opt) in options.iter().enumerate() {
                let label = opt
                    .get("label")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("?");
                let _ = write!(output, "\n  {}. {label}", i + 1);
                if let Some(desc) = opt.get("description").and_then(serde_json::Value::as_str) {
                    let _ = write!(output, " — {desc}");
                }
            }
        }

        // Return as the tool result — the LLM will present this to the user
        // and wait for their response in the next turn.
        Ok(ToolResult::success(output))
    }
}
