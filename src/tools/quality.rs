use cortex_sdk::{Tool, ToolCapabilities, ToolError, ToolResult};
use ignore::WalkBuilder;
use regex::Regex;
use std::path::{Path, PathBuf};

const MAX_SCAN_FILES: usize = 10_000;
const MAX_FILE_BYTES: u64 = 512 * 1024;

fn root_input(input: &serde_json::Value) -> PathBuf {
    input
        .get("path")
        .and_then(serde_json::Value::as_str)
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
}

fn ensure_dir(path: &Path) -> Result<(), ToolError> {
    if path.is_dir() {
        Ok(())
    } else {
        Err(ToolError::InvalidInput(format!(
            "not a directory: {}",
            path.display()
        )))
    }
}

pub struct SecretScanTool;

impl Tool for SecretScanTool {
    fn name(&self) -> &'static str {
        "secret_scan"
    }

    fn description(&self) -> &'static str {
        "Scan a workspace for likely secrets and credential leaks.\n\n\
         Use before commits, releases, and when handling configuration files. \
         This is heuristic local scanning, not a complete security audit."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Workspace root (default: current directory)" },
                "max_files": { "type": "integer", "description": "Maximum files to scan (default: 10000)" }
            }
        })
    }

    fn capabilities(&self) -> ToolCapabilities {
        super::caps([super::read_file_effect("workspace files")])
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let root = root_input(&input);
        ensure_dir(&root)?;
        let max_files = input
            .get("max_files")
            .and_then(serde_json::Value::as_u64)
            .map(usize::try_from)
            .transpose()
            .map_err(|_| ToolError::InvalidInput("max_files is too large".into()))?
            .unwrap_or(MAX_SCAN_FILES);
        let findings = scan_secrets(&root, max_files)?;
        if findings.is_empty() {
            Ok(ToolResult::success("No likely secrets found".to_string()))
        } else {
            Ok(ToolResult::error(format!(
                "Potential secrets found:\n{}",
                findings.join("\n")
            )))
        }
    }
}

fn scan_secrets(root: &Path, max_files: usize) -> Result<Vec<String>, ToolError> {
    let patterns = [
        (
            "generic secret assignment",
            Regex::new(
                r#"(?i)\b(api[_-]?key|secret|token|password)\b\s*[:=]\s*['"][^'"]{12,}['"]"#,
            ),
        ),
        (
            "private key marker",
            Regex::new(r"-----BEGIN (RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----"),
        ),
        (
            "github token",
            Regex::new(r"\bgh[pousr]_[A-Za-z0-9_]{20,}\b"),
        ),
        ("aws access key", Regex::new(r"\bAKIA[0-9A-Z]{16}\b")),
    ];
    let patterns = patterns
        .into_iter()
        .map(|(name, pattern)| pattern.map(|pattern| (name, pattern)))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ToolError::ExecutionFailed(format!("secret pattern failed: {e}")))?;

    let mut findings = Vec::new();
    let mut scanned = 0usize;
    for entry in WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .build()
        .flatten()
    {
        let path = entry.path();
        if !path.is_file() || is_binary_like(path) {
            continue;
        }
        scanned += 1;
        if scanned > max_files {
            break;
        }
        let Ok(metadata) = path.metadata() else {
            continue;
        };
        if metadata.len() > MAX_FILE_BYTES {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        for (line_index, line) in text.lines().enumerate() {
            for (name, pattern) in &patterns {
                if pattern.is_match(line) {
                    let display = path.strip_prefix(root).unwrap_or(path).display();
                    findings.push(format!("{display}:{} {name}", line_index + 1));
                }
            }
        }
    }
    findings.sort();
    findings.dedup();
    Ok(findings)
}

fn is_binary_like(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some(
            "png"
                | "jpg"
                | "jpeg"
                | "gif"
                | "webp"
                | "pdf"
                | "zip"
                | "gz"
                | "xz"
                | "zst"
                | "sqlite"
                | "db"
                | "so"
                | "dylib"
                | "wasm"
        )
    )
}

pub struct QualityGateTool;

impl Tool for QualityGateTool {
    fn name(&self) -> &'static str {
        "quality_gate"
    }

    fn description(&self) -> &'static str {
        "Produce a release/readiness gate from local project signals.\n\n\
         Use before commits and releases to check git cleanliness, test command \
         discoverability, CI presence, dependency manifests, and secret risk."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Workspace root (default: current directory)" }
            }
        })
    }

    fn capabilities(&self) -> ToolCapabilities {
        super::caps([
            super::read_file_effect("workspace files"),
            super::run_process_effect("git status"),
        ])
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let root = root_input(&input);
        ensure_dir(&root)?;
        let report = quality_report(&root)?;
        Ok(ToolResult::success(report))
    }
}

fn quality_report(root: &Path) -> Result<String, ToolError> {
    let mut lines = vec![format!("Quality gate: {}", root.display())];
    lines.push(format!("Git: {}", git_state(root)));
    lines.push(format!("CI: {}", ci_state(root)));
    lines.push(format!("Tests: {}", test_state(root)));
    lines.push(format!("Dependencies: {}", dependency_state(root)));
    let secret_count = scan_secrets(root, 2_000)?.len();
    lines.push(format!(
        "Secrets: {}",
        if secret_count == 0 {
            "no likely secrets found".to_string()
        } else {
            format!("{secret_count} potential finding(s)")
        }
    ));
    Ok(lines.join("\n"))
}

fn git_state(root: &Path) -> String {
    if !root.join(".git").exists() {
        return "not a git repository".to_string();
    }
    let output = std::process::Command::new("git")
        .args(["status", "--short"])
        .current_dir(root)
        .output();
    match output {
        Ok(output) if output.status.success() && output.stdout.is_empty() => "clean".to_string(),
        Ok(output) if output.status.success() => {
            let count = String::from_utf8_lossy(&output.stdout).lines().count();
            format!("{count} changed path(s)")
        }
        Ok(output) => format!(
            "status failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        Err(error) => format!("git unavailable: {error}"),
    }
}

fn ci_state(root: &Path) -> &'static str {
    if root.join(".github/workflows").is_dir()
        || root.join(".gitlab-ci.yml").exists()
        || root.join("Jenkinsfile").exists()
    {
        "present"
    } else {
        "not detected"
    }
}

fn test_state(root: &Path) -> &'static str {
    if root.join("Cargo.toml").exists()
        || root.join("package.json").exists()
        || root.join("pyproject.toml").exists()
        || root.join("go.mod").exists()
    {
        "detectable"
    } else {
        "not detected"
    }
}

fn dependency_state(root: &Path) -> &'static str {
    if root.join("Cargo.lock").exists()
        || root.join("package-lock.json").exists()
        || root.join("pnpm-lock.yaml").exists()
        || root.join("yarn.lock").exists()
        || root.join("uv.lock").exists()
        || root.join("go.sum").exists()
    {
        "lockfile present"
    } else if root.join("Cargo.toml").exists()
        || root.join("package.json").exists()
        || root.join("pyproject.toml").exists()
        || root.join("go.mod").exists()
    {
        "manifest without detected lockfile"
    } else {
        "not detected"
    }
}
