use cortex_sdk::{Tool, ToolCapabilities, ToolError, ToolResult, ToolRuntime};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

fn parse_task_id(value: &serde_json::Value, field: &str) -> Result<u32, ToolError> {
    let raw = value
        .get(field)
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| ToolError::InvalidInput(format!("missing '{field}'")))?;
    u32::try_from(raw)
        .map_err(|_| ToolError::InvalidInput(format!("'{field}' is too large: {raw}")))
}

fn parse_task_links(values: &[serde_json::Value], field: &str) -> Result<Vec<u32>, ToolError> {
    values
        .iter()
        .map(|value| {
            let raw = value.as_u64().ok_or_else(|| {
                ToolError::InvalidInput(format!("'{field}' entries must be integers"))
            })?;
            u32::try_from(raw).map_err(|_| {
                ToolError::InvalidInput(format!("'{field}' entry is too large: {raw}"))
            })
        })
        .collect()
}

// ── Task data model ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskItem {
    pub id: u32,
    pub subject: String,
    pub description: String,
    pub status: TaskStatus,
    #[serde(default)]
    pub blocked_by: Vec<u32>,
    #[serde(default)]
    pub blocks: Vec<u32>,
    #[serde(default)]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Deleted,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Completed => write!(f, "completed"),
            Self::Deleted => write!(f, "deleted"),
        }
    }
}

// ── Shared task store ───────────────────────────────────────

#[derive(Default)]
pub struct TaskStore {
    tasks: Vec<TaskItem>,
    next_id: u32,
}

impl TaskStore {
    fn create(&mut self, subject: &str, description: &str) -> &TaskItem {
        self.next_id += 1;
        let task = TaskItem {
            id: self.next_id,
            subject: subject.into(),
            description: description.into(),
            status: TaskStatus::Pending,
            blocked_by: Vec::new(),
            blocks: Vec::new(),
            metadata: serde_json::Map::new(),
        };
        self.tasks.push(task);
        let last_index = self.tasks.len().saturating_sub(1);
        &self.tasks[last_index]
    }

    fn get_mut(&mut self, id: u32) -> Option<&mut TaskItem> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    fn list(&self) -> Vec<&TaskItem> {
        self.tasks
            .iter()
            .filter(|t| t.status != TaskStatus::Deleted)
            .collect()
    }
}

#[derive(Default)]
pub struct NamespacedTaskStore {
    namespaces: std::collections::HashMap<String, TaskStore>,
}

impl NamespacedTaskStore {
    fn namespace_mut(&mut self, namespace: &str) -> &mut TaskStore {
        self.namespaces.entry(namespace.to_string()).or_default()
    }

    fn namespace(&self, namespace: &str) -> Option<&TaskStore> {
        self.namespaces.get(namespace)
    }
}

pub type SharedTaskStore = Arc<Mutex<NamespacedTaskStore>>;

#[must_use]
pub fn new_task_store() -> SharedTaskStore {
    Arc::new(Mutex::new(NamespacedTaskStore::default()))
}

fn runtime_namespace(runtime: &dyn ToolRuntime) -> String {
    let invocation = runtime.invocation();
    invocation
        .actor
        .as_deref()
        .or(invocation.session_id.as_deref())
        .unwrap_or("global")
        .to_string()
}

// ── TaskCreate tool ─────────────────────────────────────────

pub struct TaskCreateTool {
    store: SharedTaskStore,
}

impl TaskCreateTool {
    pub const fn new(store: SharedTaskStore) -> Self {
        Self { store }
    }
}

impl Tool for TaskCreateTool {
    fn name(&self) -> &'static str {
        "task_create"
    }

    fn description(&self) -> &'static str {
        "Create a task for tracking work progress.\n\n\
         Use when breaking complex work into steps. Each task has a subject, description, \
         and status (pending → in_progress → completed). Tasks can block each other \
         via blocked_by/blocks relationships."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "subject": {
                    "type": "string",
                    "description": "Brief imperative title (e.g., 'Fix authentication bug')"
                },
                "description": {
                    "type": "string",
                    "description": "What needs to be done"
                }
            },
            "required": ["subject", "description"]
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
        let namespace = runtime_namespace(runtime);
        runtime.emit_observer(
            Some("tasks"),
            &format!("creating task in namespace '{namespace}'"),
        );
        self.create_in_namespace(&input, &namespace)
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities {
            emits_observer_text: true,
            ..ToolCapabilities::default()
        }
    }
}

impl TaskCreateTool {
    fn create_in_namespace(
        &self,
        input: &serde_json::Value,
        namespace: &str,
    ) -> Result<ToolResult, ToolError> {
        let subject = input
            .get("subject")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("missing 'subject'".into()))?;
        let description = input
            .get("description")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");

        let task = self
            .store
            .lock()
            .map_err(|e| ToolError::ExecutionFailed(format!("lock error: {e}")))?
            .namespace_mut(namespace)
            .create(subject, description)
            .clone();
        let id = task.id;
        Ok(ToolResult::success(format!(
            "Task #{id} created in {namespace}: {subject}"
        )))
    }
}

// ── TaskList tool ───────────────────────────────────────────

pub struct TaskListTool {
    store: SharedTaskStore,
}

impl TaskListTool {
    pub const fn new(store: SharedTaskStore) -> Self {
        Self { store }
    }
}

impl Tool for TaskListTool {
    fn name(&self) -> &'static str {
        "task_list"
    }

    fn description(&self) -> &'static str {
        "List all tasks with their status."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object", "properties": {}})
    }

    fn execute(&self, _input: serde_json::Value) -> Result<ToolResult, ToolError> {
        self.list_in_namespace("global")
    }

    fn execute_with_runtime(
        &self,
        _input: serde_json::Value,
        runtime: &dyn ToolRuntime,
    ) -> Result<ToolResult, ToolError> {
        let namespace = runtime_namespace(runtime);
        self.list_in_namespace(&namespace)
    }
}

impl TaskListTool {
    fn list_in_namespace(&self, namespace: &str) -> Result<ToolResult, ToolError> {
        let store = self
            .store
            .lock()
            .map_err(|e| ToolError::ExecutionFailed(format!("lock error: {e}")))?;
        let tasks: Vec<TaskItem> = store.namespace(namespace).map_or_else(Vec::new, |tasks| {
            tasks.list().into_iter().cloned().collect()
        });
        drop(store);
        if tasks.is_empty() {
            return Ok(ToolResult::success(format!(
                "No tasks in namespace '{namespace}'"
            )));
        }
        let mut lines = Vec::new();
        for t in tasks {
            let blocked = if t.blocked_by.is_empty() {
                String::new()
            } else {
                format!(
                    " (blocked by: {})",
                    t.blocked_by
                        .iter()
                        .map(|id| format!("#{id}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            lines.push(format!("#{} [{}] {}{blocked}", t.id, t.status, t.subject));
        }
        Ok(ToolResult::success(format!(
            "Namespace: {namespace}\n{}",
            lines.join("\n")
        )))
    }
}

// ── TaskUpdate tool ─────────────────────────────────────────

pub struct TaskUpdateTool {
    store: SharedTaskStore,
}

impl TaskUpdateTool {
    pub const fn new(store: SharedTaskStore) -> Self {
        Self { store }
    }
}

impl Tool for TaskUpdateTool {
    fn name(&self) -> &'static str {
        "task_update"
    }

    fn description(&self) -> &'static str {
        "Update a task's status, add blockers, or change details.\n\n\
         Status workflow: pending → in_progress → completed. Use 'deleted' to remove."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "integer",
                    "description": "Task ID (e.g., 1)"
                },
                "status": {
                    "type": "string",
                    "description": "New status: pending, in_progress, completed, deleted",
                    "enum": ["pending", "in_progress", "completed", "deleted"]
                },
                "add_blocked_by": {
                    "type": "array",
                    "items": {"type": "integer"},
                    "description": "Task IDs that block this task"
                },
                "add_blocks": {
                    "type": "array",
                    "items": {"type": "integer"},
                    "description": "Task IDs that this task blocks"
                },
                "subject": {
                    "type": "string",
                    "description": "New subject (optional)"
                },
                "description": {
                    "type": "string",
                    "description": "New description (optional)"
                }
            },
            "required": ["id"]
        })
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        self.update_in_namespace(&input, "global")
    }

    fn execute_with_runtime(
        &self,
        input: serde_json::Value,
        runtime: &dyn ToolRuntime,
    ) -> Result<ToolResult, ToolError> {
        let namespace = runtime_namespace(runtime);
        runtime.emit_observer(
            Some("tasks"),
            &format!("updating task state in namespace '{namespace}'"),
        );
        self.update_in_namespace(&input, &namespace)
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities {
            emits_observer_text: true,
            ..ToolCapabilities::default()
        }
    }
}

impl TaskUpdateTool {
    fn update_in_namespace(
        &self,
        input: &serde_json::Value,
        namespace: &str,
    ) -> Result<ToolResult, ToolError> {
        let id = parse_task_id(input, "id")?;

        let mut store = self
            .store
            .lock()
            .map_err(|e| ToolError::ExecutionFailed(format!("lock error: {e}")))?;

        let task = store
            .namespace_mut(namespace)
            .get_mut(id)
            .ok_or_else(|| ToolError::InvalidInput(format!("task #{id} not found")))?;

        if let Some(status_str) = input.get("status").and_then(serde_json::Value::as_str) {
            task.status = match status_str {
                "pending" => TaskStatus::Pending,
                "in_progress" => TaskStatus::InProgress,
                "completed" => TaskStatus::Completed,
                "deleted" => TaskStatus::Deleted,
                other => return Err(ToolError::InvalidInput(format!("invalid status '{other}'"))),
            };
        }

        if let Some(subject) = input.get("subject").and_then(serde_json::Value::as_str) {
            task.subject = subject.into();
        }
        if let Some(desc) = input.get("description").and_then(serde_json::Value::as_str) {
            task.description = desc.into();
        }

        if let Some(blockers) = input
            .get("add_blocked_by")
            .and_then(serde_json::Value::as_array)
        {
            for blocker_id in parse_task_links(blockers, "add_blocked_by")? {
                if !task.blocked_by.contains(&blocker_id) {
                    task.blocked_by.push(blocker_id);
                }
            }
        }

        if let Some(blocks) = input
            .get("add_blocks")
            .and_then(serde_json::Value::as_array)
        {
            for blocked_id in parse_task_links(blocks, "add_blocks")? {
                if !task.blocks.contains(&blocked_id) {
                    task.blocks.push(blocked_id);
                }
            }
        }

        let result = format!("Task #{id} updated: [{}] {}", task.status, task.subject);
        drop(store);
        Ok(ToolResult::success(result))
    }
}
