#[path = "support/result.rs"]
mod result_support;
#[path = "support/temp.rs"]
mod temp_support;

use cortex_plugin_dev::tools::{ReplaceInFileTool, WriteFileTool};
use cortex_sdk::Tool;
use result_support::ResultTestExt;
use temp_support::temp_dir;

#[test]
fn replace_literal_guard() {
    let tmp = temp_dir("cortex-dev-file-ops");
    let path = tmp.join("a.txt");
    std::fs::write(&path, "alpha beta alpha").or_panic();
    let tool = ReplaceInFileTool;
    let result = tool
        .execute(serde_json::json!({
            "path": path,
            "old": "alpha",
            "new": "gamma",
            "expected_replacements": 2
        }))
        .or_panic();
    assert!(result.output.contains("replaced 2"));
    assert_eq!(std::fs::read_to_string(path).or_panic(), "gamma beta gamma");
    std::fs::remove_dir_all(tmp).or_panic();
}

#[test]
fn write_requires_overwrite() {
    let tmp = temp_dir("cortex-dev-file-ops");
    let path = tmp.join("a.txt");
    std::fs::write(&path, "old").or_panic();
    let tool = WriteFileTool;
    assert!(
        tool.execute(serde_json::json!({"path": path, "content": "new"}))
            .is_err()
    );
    std::fs::remove_dir_all(tmp).or_panic();
}
