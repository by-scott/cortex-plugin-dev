use cortex_sdk::{Tool, ToolError, ToolResult};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Shared plan mode state. When active, signals to the LLM that it should
/// explore and design before executing. The runtime doesn't enforce this —
/// it's a cognitive signal, not a permission gate.
pub type PlanModeState = Arc<AtomicBool>;

#[must_use]
pub fn new_plan_state() -> PlanModeState {
    Arc::new(AtomicBool::new(false))
}

pub struct EnterPlanModeTool {
    state: PlanModeState,
}

impl EnterPlanModeTool {
    pub const fn new(state: PlanModeState) -> Self {
        Self { state }
    }
}

impl Tool for EnterPlanModeTool {
    fn name(&self) -> &'static str {
        "enter_plan_mode"
    }

    fn description(&self) -> &'static str {
        "Enter plan mode for non-trivial implementation tasks.\n\n\
         In plan mode:\n\
         1. Explore the codebase with read-only tools (read, glob, grep)\n\
         2. Design an implementation approach\n\
         3. Present the plan for user approval\n\
         4. Exit plan mode (via exit_plan_mode) to begin implementation\n\n\
         Use proactively before writing significant code. Getting alignment \
         upfront prevents wasted effort."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "reason": {
                    "type": "string",
                    "description": "Why planning is needed before execution"
                }
            }
        })
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        self.state.store(true, Ordering::Relaxed);
        let reason = input
            .get("reason")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("non-trivial task");
        Ok(ToolResult::success(format!(
            "Plan mode active. Reason: {reason}\n\
             Explore the codebase, design an approach, then present your plan.\n\
             Use exit_plan_mode when the plan is ready for review."
        )))
    }
}

pub struct ExitPlanModeTool {
    state: PlanModeState,
}

impl ExitPlanModeTool {
    pub const fn new(state: PlanModeState) -> Self {
        Self { state }
    }
}

impl Tool for ExitPlanModeTool {
    fn name(&self) -> &'static str {
        "exit_plan_mode"
    }

    fn description(&self) -> &'static str {
        "Exit plan mode and signal the plan is ready for user review.\n\n\
         The plan should already be written or presented before calling this. \
         After exiting, you can proceed with implementation."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "summary": {
                    "type": "string",
                    "description": "One-line summary of the plan"
                }
            }
        })
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let was_active = self.state.swap(false, Ordering::Relaxed);
        if !was_active {
            return Ok(ToolResult::success("Not in plan mode.".to_string()));
        }
        let summary = input
            .get("summary")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Plan complete");
        Ok(ToolResult::success(format!(
            "Plan mode exited. {summary}\nProceeding to implementation."
        )))
    }
}
