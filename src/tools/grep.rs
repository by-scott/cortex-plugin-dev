use cortex_sdk::{Tool, ToolError, ToolResult};
use std::path::Path;

fn parse_usize_field(
    input: &serde_json::Value,
    field: &str,
    default: usize,
) -> Result<usize, ToolError> {
    input
        .get(field)
        .and_then(serde_json::Value::as_u64)
        .map_or(Ok(default), |value| {
            usize::try_from(value).map_err(|_| {
                ToolError::InvalidInput(format!(
                    "'{field}' is too large for this platform: {value}"
                ))
            })
        })
}

fn push_context_lines(
    output: &mut Vec<String>,
    path: Option<&Path>,
    lines: &[&str],
    match_index: usize,
    context_lines: usize,
) {
    let start = match_index.saturating_sub(context_lines);
    let end = (match_index + context_lines + 1).min(lines.len());

    for (line_index, line) in lines.iter().enumerate().take(end).skip(start) {
        let prefix = if line_index == match_index { ">" } else { " " };
        match path {
            Some(path) => output.push(format!(
                "{}:{}:{} {}",
                path.to_string_lossy(),
                line_index + 1,
                prefix,
                line
            )),
            None => output.push(format!("{}:{prefix} {}", line_index + 1, line)),
        }
    }

    if context_lines > 0 {
        output.push("--".into());
    }
}

fn search_single_file(
    path: &Path,
    re: &regex::Regex,
    files_only: bool,
    context_lines: usize,
    limit: usize,
) -> Result<ToolResult, ToolError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| ToolError::ExecutionFailed(format!("cannot read {}: {e}", path.display())))?;

    if files_only {
        if re.is_match(&content) {
            return Ok(ToolResult::success(path.to_string_lossy().into_owned()));
        }
        return Ok(ToolResult::success("No matches".to_string()));
    }

    let lines: Vec<&str> = content.lines().collect();
    let mut output = Vec::new();
    let mut count = 0;

    for (line_index, line) in lines.iter().enumerate() {
        if !re.is_match(line) {
            continue;
        }

        push_context_lines(&mut output, None, &lines, line_index, context_lines);
        count += 1;
        if count >= limit {
            break;
        }
    }

    if output.is_empty() {
        Ok(ToolResult::success("No matches".to_string()))
    } else {
        Ok(ToolResult::success(output.join("\n")))
    }
}

fn build_glob_matcher(
    base: &Path,
    file_glob: Option<&str>,
) -> Result<Option<ignore::overrides::Override>, ToolError> {
    let Some(glob) = file_glob else {
        return Ok(None);
    };

    let mut builder = ignore::overrides::OverrideBuilder::new(base);
    builder
        .add(glob)
        .map_err(|e| ToolError::InvalidInput(format!("invalid glob: {e}")))?;
    builder
        .build()
        .map(Some)
        .map_err(|e| ToolError::InvalidInput(format!("glob build: {e}")))
}

fn append_match_from_file(
    output: &mut Vec<String>,
    path: &Path,
    re: &regex::Regex,
    files_only: bool,
    context_lines: usize,
    limit: usize,
) -> (usize, bool) {
    let Ok(content) = std::fs::read_to_string(path) else {
        return (0, false);
    };

    let lines: Vec<&str> = content.lines().collect();
    let mut match_count = 0;
    let mut file_has_match = false;

    for (line_index, line) in lines.iter().enumerate() {
        if !re.is_match(line) {
            continue;
        }

        file_has_match = true;
        if files_only {
            output.push(path.to_string_lossy().into_owned());
            return (1, true);
        }

        push_context_lines(output, Some(path), &lines, line_index, context_lines);
        match_count += 1;
        if match_count >= limit {
            return (match_count, true);
        }
    }

    (match_count, file_has_match)
}

pub struct GrepTool;

impl Tool for GrepTool {
    fn name(&self) -> &'static str {
        "grep"
    }

    fn description(&self) -> &'static str {
        "Search file contents with regex. Fast, respects .gitignore.\n\n\
         Use instead of `bash` with `grep` or `rg` — faster, structured output.\n\
         Supports full regex syntax. Returns matching lines with file paths and line numbers."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "File or directory to search (default: current directory)"
                },
                "glob": {
                    "type": "string",
                    "description": "File glob filter (e.g., \"*.rs\", \"*.{ts,tsx}\")"
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Case-insensitive matching (default: false)"
                },
                "files_only": {
                    "type": "boolean",
                    "description": "Only return file paths, not matching lines (default: false)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum results (default: 100)"
                },
                "context": {
                    "type": "integer",
                    "description": "Lines of context around each match (default: 0)"
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

        let file_glob = input.get("glob").and_then(serde_json::Value::as_str);

        let case_insensitive = input
            .get("case_insensitive")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        let files_only = input
            .get("files_only")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        let limit = parse_usize_field(&input, "limit", 100)?;
        let context_lines = parse_usize_field(&input, "context", 0)?;

        let re = regex::RegexBuilder::new(pattern)
            .case_insensitive(case_insensitive)
            .build()
            .map_err(|e| ToolError::InvalidInput(format!("invalid regex: {e}")))?;

        let base = std::path::Path::new(base_path);

        if base.is_file() {
            return search_single_file(base, &re, files_only, context_lines, limit);
        }

        if !base.is_dir() {
            return Err(ToolError::InvalidInput(format!(
                "path does not exist: {base_path}"
            )));
        }

        let glob_matcher = build_glob_matcher(base, file_glob)?;

        let mut walker = ignore::WalkBuilder::new(base);
        walker.hidden(false).git_ignore(true);

        let mut output = Vec::new();
        let mut match_count = 0;
        let mut file_count = 0;

        for entry in walker.build().flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            if let Some(ref matcher) = glob_matcher {
                let rel = path.strip_prefix(base).unwrap_or(path);
                if !matcher.matched(rel, false).is_whitelist() {
                    continue;
                }
            }

            let remaining = limit.saturating_sub(match_count);
            let (matches_in_file, file_has_match) = append_match_from_file(
                &mut output,
                path,
                &re,
                files_only,
                context_lines,
                remaining,
            );
            if file_has_match {
                file_count += 1;
            }
            match_count += matches_in_file;
            if !files_only && match_count >= limit {
                output.push(format!("(truncated at {limit} matches)"));
                return Ok(ToolResult::success(output.join("\n")));
            }
        }

        if output.is_empty() {
            return Ok(ToolResult::success(format!(
                "No matches for /{pattern}/ in {base_path}"
            )));
        }

        let summary = if files_only {
            format!("{file_count} files")
        } else {
            format!("{match_count} matches in {file_count} files")
        };
        output.push(format!("({summary})"));

        Ok(ToolResult::success(output.join("\n")))
    }
}
