use cortex_sdk::{Tool, ToolCapabilities, ToolError, ToolResult, ToolRuntime};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type TeamStore = Arc<Mutex<HashMap<String, HashMap<String, Vec<String>>>>>;

#[must_use]
pub fn new_team_store() -> TeamStore {
    Arc::new(Mutex::new(HashMap::new()))
}

pub struct TeamCreateTool {
    store: TeamStore,
}

impl TeamCreateTool {
    pub const fn new(store: TeamStore) -> Self {
        Self { store }
    }
}

impl Tool for TeamCreateTool {
    fn name(&self) -> &'static str {
        "team_create"
    }

    fn description(&self) -> &'static str {
        "Create a named agent team for parallel work.\n\n\
         Members are agent names that can coordinate via send_message."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Team name" },
                "members": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Agent names in the team"
                }
            },
            "required": ["name", "members"]
        })
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        self.create_in_namespace(&input, "global")
    }

    fn execute_with_runtime(
        &self,
        input: serde_json::Value,
        runtime: &dyn ToolRuntime,
    ) -> Result<ToolResult, ToolError> {
        let namespace = super::runtime::namespace(runtime);
        super::runtime::observe(
            runtime,
            "teams",
            format!("creating team state in namespace '{namespace}'"),
        );
        self.create_in_namespace(&input, &namespace)
    }

    fn capabilities(&self) -> ToolCapabilities {
        super::observer_caps([super::delegate_effect("team state")])
    }
}

impl TeamCreateTool {
    fn create_in_namespace(
        &self,
        input: &serde_json::Value,
        namespace: &str,
    ) -> Result<ToolResult, ToolError> {
        let name = input
            .get("name")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("missing 'name'".into()))?;
        let members: Vec<String> = input
            .get("members")
            .and_then(serde_json::Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        if members.is_empty() {
            return Err(ToolError::InvalidInput(
                "team needs at least one member".into(),
            ));
        }

        self.store
            .lock()
            .map_err(|e| ToolError::ExecutionFailed(format!("lock: {e}")))?
            .entry(namespace.to_string())
            .or_default()
            .insert(name.into(), members.clone());

        Ok(ToolResult::success(format!(
            "Team '{name}' created in {namespace} with {} members: {}",
            members.len(),
            members.join(", ")
        )))
    }
}

pub struct TeamDeleteTool {
    store: TeamStore,
}

impl TeamDeleteTool {
    pub const fn new(store: TeamStore) -> Self {
        Self { store }
    }
}

impl Tool for TeamDeleteTool {
    fn name(&self) -> &'static str {
        "team_delete"
    }

    fn description(&self) -> &'static str {
        "Delete a named agent team."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Team name to delete" }
            },
            "required": ["name"]
        })
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        self.delete_in_namespace(&input, "global")
    }

    fn execute_with_runtime(
        &self,
        input: serde_json::Value,
        runtime: &dyn ToolRuntime,
    ) -> Result<ToolResult, ToolError> {
        let namespace = super::runtime::namespace(runtime);
        self.delete_in_namespace(&input, &namespace)
    }

    fn capabilities(&self) -> ToolCapabilities {
        super::caps([super::delegate_effect("team state")])
    }
}

impl TeamDeleteTool {
    fn delete_in_namespace(
        &self,
        input: &serde_json::Value,
        namespace: &str,
    ) -> Result<ToolResult, ToolError> {
        let name = input
            .get("name")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("missing 'name'".into()))?;

        let mut store = self
            .store
            .lock()
            .map_err(|e| ToolError::ExecutionFailed(format!("lock: {e}")))?;

        if store
            .get_mut(namespace)
            .is_some_and(|teams| teams.remove(name).is_some())
        {
            Ok(ToolResult::success(format!(
                "Team '{name}' deleted from {namespace}"
            )))
        } else {
            Ok(ToolResult::error(format!(
                "Team '{name}' not found in {namespace}"
            )))
        }
    }
}
