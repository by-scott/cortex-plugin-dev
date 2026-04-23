use std::sync::{Arc, Mutex};

pub struct TestRuntime {
    invocation: cortex_sdk::InvocationContext,
}

impl TestRuntime {
    #[must_use]
    pub fn new(tool_name: &str, session_id: &str, actor: &str) -> Self {
        Self {
            invocation: cortex_sdk::InvocationContext {
                tool_name: tool_name.to_string(),
                session_id: Some(session_id.to_string()),
                actor: Some(actor.to_string()),
                source: Some("rpc".to_string()),
                execution_scope: cortex_sdk::ExecutionScope::Foreground,
            },
        }
    }
}

impl cortex_sdk::ToolRuntime for TestRuntime {
    fn invocation(&self) -> &cortex_sdk::InvocationContext {
        &self.invocation
    }

    fn emit_progress(&self, _message: &str) {}

    fn emit_observer(&self, _source: Option<&str>, _content: &str) {}
}

#[must_use]
pub fn notes_store() -> Arc<Mutex<std::collections::HashMap<String, Vec<String>>>> {
    Arc::new(Mutex::new(std::collections::HashMap::new()))
}
