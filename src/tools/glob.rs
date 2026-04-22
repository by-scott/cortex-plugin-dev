use cortex_sdk::{Tool, ToolError, ToolResult};
use std::cmp::Reverse;

fn parse_limit(input: &serde_json::Value) -> Result<usize, ToolError> {
    input
        .get("limit")
        .and_then(serde_json::Value::as_u64)
        .map_or(Ok(200), |value| {
            usize::try_from(value).map_err(|_| {
                ToolError::InvalidInput(format!("limit is too large for this platform: {value}"))
            })
        })
}

pub struct GlobTool;

impl Tool for GlobTool {
    fn name(&self) -> &'static str {
        "glob"
    }

    fn description(&self) -> &'static str {
        "Find files by glob pattern. Fast, respects .gitignore.\n\n\
         Use instead of `bash` with `find` — faster, safer, and ignores build artifacts.\n\
         Returns paths sorted by modification time (newest first)."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern (e.g., \"**/*.rs\", \"src/**/*.ts\", \"*.md\")"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search in (default: current directory)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results (default: 200)"
                }
            },
            "required": ["pattern"]
        })
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let pattern = input
            .get("pattern")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("missing 'pattern'".into()))?;

        let base_path = input
            .get("path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(".");

        let limit = parse_limit(&input)?;

        let base = std::path::Path::new(base_path);
        if !base.exists() {
            return Err(ToolError::InvalidInput(format!(
                "directory does not exist: {base_path}"
            )));
        }

        // Use the `ignore` crate for .gitignore-aware walking
        let mut builder = ignore::WalkBuilder::new(base);
        builder.hidden(false).git_ignore(true).git_global(true);

        let glob_matcher = ignore::overrides::OverrideBuilder::new(base)
            .add(pattern)
            .map_err(|e| ToolError::InvalidInput(format!("invalid glob pattern: {e}")))?
            .build()
            .map_err(|e| ToolError::InvalidInput(format!("glob build error: {e}")))?;

        let mut matches: Vec<(std::path::PathBuf, std::time::SystemTime)> = Vec::new();

        for entry in builder.build().flatten() {
            let path = entry.path();
            if path.is_file()
                && glob_matcher
                    .matched(path.strip_prefix(base).unwrap_or(path), false)
                    .is_whitelist()
            {
                let mtime = path
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                matches.push((path.to_path_buf(), mtime));
            }
        }

        // Sort by modification time, newest first
        matches.sort_by_key(|entry| Reverse(entry.1));
        matches.truncate(limit);

        if matches.is_empty() {
            return Ok(ToolResult::success(format!(
                "No files matching '{pattern}' in {base_path}"
            )));
        }

        let result: Vec<String> = matches
            .iter()
            .map(|(p, _)| p.to_string_lossy().into_owned())
            .collect();

        Ok(ToolResult::success(result.join("\n")))
    }
}
