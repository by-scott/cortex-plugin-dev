use cortex_sdk::{Tool, ToolCapabilities, ToolError, ToolResult};
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_MAX_BYTES: u64 = 256 * 1024;
const HARD_MAX_BYTES: u64 = 4 * 1024 * 1024;

fn string_input<'a>(input: &'a serde_json::Value, key: &str) -> Result<&'a str, ToolError> {
    input
        .get(key)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ToolError::InvalidInput(format!("missing '{key}'")))
}

fn optional_bool(input: &serde_json::Value, key: &str) -> bool {
    input
        .get(key)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn optional_u64(input: &serde_json::Value, key: &str, default: u64) -> u64 {
    input
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(default)
}

fn normalize_path(path: &str) -> Result<PathBuf, ToolError> {
    let path = PathBuf::from(path);
    if path.as_os_str().is_empty() {
        return Err(ToolError::InvalidInput("path must not be empty".into()));
    }
    Ok(path)
}

fn read_text_file(path: &Path, max_bytes: u64) -> Result<String, ToolError> {
    let metadata = fs::metadata(path)
        .map_err(|e| ToolError::ExecutionFailed(format!("cannot stat {}: {e}", path.display())))?;
    if !metadata.is_file() {
        return Err(ToolError::InvalidInput(format!(
            "not a regular file: {}",
            path.display()
        )));
    }
    if metadata.len() > max_bytes {
        return Err(ToolError::InvalidInput(format!(
            "file is {} bytes, above max_bytes {max_bytes}: {}",
            metadata.len(),
            path.display()
        )));
    }
    fs::read_to_string(path).map_err(|e| {
        ToolError::ExecutionFailed(format!("cannot read {} as UTF-8: {e}", path.display()))
    })
}

fn line_window(text: &str, start_line: usize, limit_lines: usize) -> String {
    let start = start_line.saturating_sub(1);
    text.lines()
        .enumerate()
        .skip(start)
        .take(limit_lines)
        .map(|(index, line)| format!("{:>6}  {line}", index + 1))
        .collect::<Vec<_>>()
        .join("\n")
}

pub struct ReadFileTool;

impl Tool for ReadFileTool {
    fn name(&self) -> &'static str {
        "read_file"
    }

    fn description(&self) -> &'static str {
        "Read a UTF-8 text file with optional line windowing.\n\n\
         Use before editing to inspect exact content. Prefer this over shell \
         commands for ordinary source reads because it returns stable line numbers \
         and enforces size limits."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path to read" },
                "start_line": { "type": "integer", "description": "1-based first line (default: 1)" },
                "limit_lines": { "type": "integer", "description": "Maximum lines to return (default: 200)" },
                "max_bytes": { "type": "integer", "description": "Maximum file size to read (default: 262144, hard max: 4194304)" }
            },
            "required": ["path"]
        })
    }

    fn capabilities(&self) -> ToolCapabilities {
        super::caps([super::read_file_effect("path")])
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let path = normalize_path(string_input(&input, "path")?)?;
        let start_line = usize::try_from(optional_u64(&input, "start_line", 1))
            .map_err(|_| ToolError::InvalidInput("start_line is too large".into()))?;
        let limit_lines = usize::try_from(optional_u64(&input, "limit_lines", 200))
            .map_err(|_| ToolError::InvalidInput("limit_lines is too large".into()))?;
        let max_bytes = optional_u64(&input, "max_bytes", DEFAULT_MAX_BYTES).min(HARD_MAX_BYTES);
        let text = read_text_file(&path, max_bytes)?;
        let total_lines = text.lines().count();
        let body = line_window(&text, start_line.max(1), limit_lines.max(1));
        Ok(ToolResult::success(format!(
            "{} ({} lines)\n{}",
            path.display(),
            total_lines,
            body
        )))
    }
}

pub struct WriteFileTool;

impl Tool for WriteFileTool {
    fn name(&self) -> &'static str {
        "write_file"
    }

    fn description(&self) -> &'static str {
        "Write or append a UTF-8 text file with explicit overwrite controls.\n\n\
         Use for creating files or controlled full-file rewrites. For surgical \
         edits, prefer `replace_in_file`."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path to write" },
                "content": { "type": "string", "description": "UTF-8 content" },
                "overwrite": { "type": "boolean", "description": "Allow replacing an existing file (default: false)" },
                "append": { "type": "boolean", "description": "Append instead of replacing (default: false)" },
                "create_dirs": { "type": "boolean", "description": "Create parent directories (default: false)" }
            },
            "required": ["path", "content"]
        })
    }

    fn capabilities(&self) -> ToolCapabilities {
        super::caps([super::write_file_effect("path")])
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let path = normalize_path(string_input(&input, "path")?)?;
        let content = string_input(&input, "content")?;
        let overwrite = optional_bool(&input, "overwrite");
        let append = optional_bool(&input, "append");
        let create_dirs = optional_bool(&input, "create_dirs");

        if create_dirs
            && let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent).map_err(|e| {
                ToolError::ExecutionFailed(format!("cannot create {}: {e}", parent.display()))
            })?;
        }

        if path.exists() && !overwrite && !append {
            return Err(ToolError::InvalidInput(format!(
                "{} exists; set overwrite=true or append=true",
                path.display()
            )));
        }

        if append {
            use std::io::Write;
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .map_err(|e| {
                    ToolError::ExecutionFailed(format!("cannot open {}: {e}", path.display()))
                })?;
            file.write_all(content.as_bytes()).map_err(|e| {
                ToolError::ExecutionFailed(format!("cannot append {}: {e}", path.display()))
            })?;
        } else {
            fs::write(&path, content).map_err(|e| {
                ToolError::ExecutionFailed(format!("cannot write {}: {e}", path.display()))
            })?;
        }

        Ok(ToolResult::success(format!(
            "wrote {} bytes to {}",
            content.len(),
            path.display()
        )))
    }
}

pub struct ReplaceInFileTool;

impl Tool for ReplaceInFileTool {
    fn name(&self) -> &'static str {
        "replace_in_file"
    }

    fn description(&self) -> &'static str {
        "Perform a literal or regex replacement inside a UTF-8 text file.\n\n\
         Use for precise edits after reading the file. The optional \
         expected_replacements guard prevents accidental broad edits."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path to edit" },
                "old": { "type": "string", "description": "Literal text or regex pattern to replace" },
                "new": { "type": "string", "description": "Replacement text" },
                "regex": { "type": "boolean", "description": "Treat old as regex (default: false)" },
                "expected_replacements": { "type": "integer", "description": "Fail unless this exact number of replacements is made" },
                "max_bytes": { "type": "integer", "description": "Maximum file size to edit (default: 262144, hard max: 4194304)" }
            },
            "required": ["path", "old", "new"]
        })
    }

    fn capabilities(&self) -> ToolCapabilities {
        super::caps([
            super::read_file_effect("path"),
            super::write_file_effect("path"),
        ])
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let path = normalize_path(string_input(&input, "path")?)?;
        let old = string_input(&input, "old")?;
        let new = string_input(&input, "new")?;
        let regex_mode = optional_bool(&input, "regex");
        let expected = input
            .get("expected_replacements")
            .and_then(serde_json::Value::as_u64)
            .map(usize::try_from)
            .transpose()
            .map_err(|_| ToolError::InvalidInput("expected_replacements is too large".into()))?;
        let max_bytes = optional_u64(&input, "max_bytes", DEFAULT_MAX_BYTES).min(HARD_MAX_BYTES);

        if old.is_empty() {
            return Err(ToolError::InvalidInput("old must not be empty".into()));
        }

        let before = read_text_file(&path, max_bytes)?;
        let (after, count) = if regex_mode {
            let pattern = regex::Regex::new(old)
                .map_err(|e| ToolError::InvalidInput(format!("invalid regex: {e}")))?;
            let count = pattern.find_iter(&before).count();
            (pattern.replace_all(&before, new).into_owned(), count)
        } else {
            let count = before.matches(old).count();
            (before.replace(old, new), count)
        };

        if let Some(expected) = expected
            && count != expected
        {
            return Err(ToolError::InvalidInput(format!(
                "replacement count {count} did not match expected_replacements {expected}"
            )));
        }
        if count == 0 {
            return Err(ToolError::InvalidInput("no replacements made".into()));
        }

        fs::write(&path, after).map_err(|e| {
            ToolError::ExecutionFailed(format!("cannot write {}: {e}", path.display()))
        })?;
        Ok(ToolResult::success(format!(
            "replaced {count} occurrence(s) in {}",
            path.display()
        )))
    }
}
