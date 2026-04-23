#[path = "support/result.rs"]
mod result_support;
#[path = "support/runtime.rs"]
mod runtime_support;

use cortex_plugin_dev::tools::{
    TaskCreateTool, TaskListTool, TeamCreateTool, TeamDeleteTool, TodoWriteTool, new_task_store,
    new_team_store,
};
use cortex_sdk::Tool;
use result_support::ResultTestExt;
use runtime_support::{TestRuntime, notes_store};

#[test]
fn task_state_is_namespaced_by_actor() {
    let store = new_task_store();
    let create = TaskCreateTool::new(store.clone());
    let list = TaskListTool::new(store);
    let scott = TestRuntime::new("task_create", "s1", "user:scott");
    let jane = TestRuntime::new("task_list", "s2", "user:jane");

    create
        .execute_with_runtime(
            serde_json::json!({"subject": "Fix auth", "description": "trace login bug"}),
            &scott,
        )
        .or_panic();

    let scott_view = list
        .execute_with_runtime(serde_json::json!({}), &scott)
        .or_panic()
        .output;
    let jane_view = list
        .execute_with_runtime(serde_json::json!({}), &jane)
        .or_panic()
        .output;

    assert!(scott_view.contains("Fix auth"));
    assert!(!jane_view.contains("Fix auth"));
}

#[test]
fn team_state_is_namespaced_by_actor() {
    let store = new_team_store();
    let create = TeamCreateTool::new(store.clone());
    let delete = TeamDeleteTool::new(store);
    let scott = TestRuntime::new("team_create", "s1", "user:scott");
    let jane = TestRuntime::new("team_delete", "s2", "user:jane");

    create
        .execute_with_runtime(
            serde_json::json!({"name": "reviewers", "members": ["alice"]}),
            &scott,
        )
        .or_panic();

    let jane_delete = delete
        .execute_with_runtime(serde_json::json!({"name": "reviewers"}), &jane)
        .or_panic();
    assert!(jane_delete.is_error);

    let scott_delete = delete
        .execute_with_runtime(serde_json::json!({"name": "reviewers"}), &scott)
        .or_panic();
    assert!(!scott_delete.is_error);
}

#[test]
fn notes_are_namespaced_by_actor() {
    let tool = TodoWriteTool::new(notes_store());
    let scott = TestRuntime::new("todo", "s1", "user:scott");
    let jane = TestRuntime::new("todo", "s2", "user:jane");

    tool.execute_with_runtime(
        serde_json::json!({"content": "remember the failing test"}),
        &scott,
    )
    .or_panic();

    let scott_notes = tool
        .execute_with_runtime(serde_json::json!({}), &scott)
        .or_panic()
        .output;
    let jane_notes = tool
        .execute_with_runtime(serde_json::json!({}), &jane)
        .or_panic()
        .output;

    assert!(scott_notes.contains("remember the failing test"));
    assert!(!jane_notes.contains("remember the failing test"));
}
