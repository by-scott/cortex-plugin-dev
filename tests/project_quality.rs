#[path = "support/result.rs"]
mod result_support;
#[path = "support/temp.rs"]
mod temp_support;

use cortex_plugin_dev::tools::{ProjectMapTool, SecretScanTool};
use cortex_sdk::Tool;
use result_support::ResultTestExt;
use temp_support::temp_dir;

#[test]
fn discovers_rust_project() {
    let tmp = temp_dir("cortex-dev-project");
    std::fs::write(tmp.join("Cargo.toml"), "[package]\nname=\"x\"\n").or_panic();
    std::fs::create_dir_all(tmp.join("src")).or_panic();
    std::fs::write(tmp.join("src/main.rs"), "fn main() {}\n").or_panic();

    let tool = ProjectMapTool;
    let result = tool
        .execute(serde_json::json!({
            "path": tmp,
            "format": "json"
        }))
        .or_panic();
    assert!(result.output.contains("rust/cargo"));
    assert!(result.output.contains("cargo test"));
}

#[test]
fn detects_secret_assignment() {
    let tmp = temp_dir("cortex-dev-quality");
    std::fs::write(tmp.join(".env"), "API_KEY=\"abcdefghijklmnopqrstuvwxyz\"\n").or_panic();

    let tool = SecretScanTool;
    let result = tool
        .execute(serde_json::json!({
            "path": tmp,
            "max_files": 100
        }))
        .or_panic();
    assert!(result.is_error);
    assert!(result.output.contains("Potential secrets found"));
}
