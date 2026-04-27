use cortex_sdk::{Tool, ToolCapabilities, ToolError, ToolResult};
use std::path::Path;
use std::process::Command;

fn line_to_index(line: u64) -> Result<usize, ToolError> {
    usize::try_from(line)
        .map(|line| line.saturating_sub(1))
        .map_err(|_| ToolError::InvalidInput(format!("line {line} is too large")))
}

/// Language server operations via CLI tools — no embedded LSP.
pub struct LspTool;

impl Tool for LspTool {
    fn name(&self) -> &'static str {
        "lsp"
    }

    fn description(&self) -> &'static str {
        "Language server operations: find definitions, references, completions.\n\n\
         Uses CLI tools (rust-analyzer, pyright, tsc) — no running LSP needed.\n\
         Commands:\n\
         - `definition <file> <line> <col>`: go to definition\n\
         - `references <file> <line> <col>`: find all references\n\
         - `hover <file> <line> <col>`: type info at position\n\
         - `check <file>`: type-check a single file"
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "enum": ["definition", "references", "hover", "check"] },
                "file": { "type": "string", "description": "File path" },
                "line": { "type": "integer", "description": "1-indexed line number" },
                "col": { "type": "integer", "description": "1-indexed column" }
            },
            "required": ["command", "file"]
        })
    }

    fn capabilities(&self) -> ToolCapabilities {
        super::caps([
            super::read_file_effect("source file"),
            super::run_process_effect("language checker"),
        ])
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let cmd = input
            .get("command")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("missing 'command'".into()))?;
        let file = input
            .get("file")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("missing 'file'".into()))?;
        let line = input
            .get("line")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1);
        let col = input
            .get("col")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1);

        let ext = std::path::Path::new(file)
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("");

        match cmd {
            "check" => run_check(file, ext),
            "definition" | "references" | "hover" => {
                // Use grep-based fallback for definition/references
                // Real LSP requires a running server; CLI tools don't expose these directly
                match cmd {
                    "definition" => find_definition(file, line, col, ext),
                    "references" => find_references(file, line, col, ext),
                    "hover" => hover_info(file, line, col, ext),
                    _ => unreachable!(),
                }
            }
            other => Err(ToolError::InvalidInput(format!("unknown: {other}"))),
        }
    }
}

fn run_check(file: &str, ext: &str) -> Result<ToolResult, ToolError> {
    let (cmd, args): (&str, Vec<&str>) = match ext {
        "rs" => ("cargo", vec!["check", "--message-format=short"]),
        "py" => ("pyright", vec![file]),
        "ts" | "tsx" => ("tsc", vec!["--noEmit", file]),
        "go" => ("go", vec!["vet", file]),
        _ => return Err(ToolError::InvalidInput(format!("unsupported: .{ext}"))),
    };

    let output = Command::new(cmd)
        .args(&args)
        .output()
        .map_err(|e| ToolError::ExecutionFailed(format!("`{cmd}` failed: {e}")))?;
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(ToolResult::success(combined.trim().to_string()))
}

fn word_at_position(file: &str, line: u64, col: u64) -> Result<String, ToolError> {
    let content = std::fs::read_to_string(file)
        .map_err(|e| ToolError::ExecutionFailed(format!("read: {e}")))?;
    let lines: Vec<&str> = content.lines().collect();
    let idx = line_to_index(line)?;
    if idx >= lines.len() {
        return Err(ToolError::InvalidInput(format!("line {line} out of range")));
    }
    let line_text = lines[idx];
    let raw_col = usize::try_from(col).unwrap_or(1).saturating_sub(1);
    let col = raw_col.min(line_text.len());
    let bytes = line_text.as_bytes();
    let mut start = col;
    while start > 0 && is_ident_byte(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = col;
    while end < bytes.len() && is_ident_byte(bytes[end]) {
        end += 1;
    }
    if start == end {
        return Err(ToolError::InvalidInput(format!(
            "no symbol at {file}:{line}:{col}"
        )));
    }
    Ok(line_text[start..end].to_string())
}

const fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn workspace_root_for(file: &str) -> &Path {
    Path::new(file)
        .ancestors()
        .find(|path| path.join(".git").exists())
        .unwrap_or_else(|| Path::new("."))
}

fn parse_file_symbols(path: &Path) -> Vec<crate::treesitter::Symbol> {
    let Ok(lang) = crate::treesitter::SupportedLanguage::from_path(path) else {
        return Vec::new();
    };
    let Ok(source) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    crate::treesitter::extract_symbols(&source, lang).unwrap_or_default()
}

fn find_definition(file: &str, line: u64, col: u64, _ext: &str) -> Result<ToolResult, ToolError> {
    let word = word_at_position(file, line, col)?;
    let root = workspace_root_for(file);
    let mut matches = Vec::new();
    let mut walker = ignore::WalkBuilder::new(root);
    walker.hidden(false).git_ignore(true).git_global(true);

    for entry in walker.build().flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        for symbol in parse_file_symbols(path) {
            if symbol.name == word {
                let signature = symbol.signature.unwrap_or_else(|| symbol.name.clone());
                matches.push(format!(
                    "{}:{}-{}: {:?} {}",
                    path.display(),
                    symbol.range.start_line,
                    symbol.range.end_line,
                    symbol.kind,
                    signature
                ));
            }
        }
    }

    if matches.is_empty() {
        return Ok(ToolResult::success(format!(
            "No definition found for `{word}` from {file}:{line}:{col}"
        )));
    }

    Ok(ToolResult::success(format!(
        "Definitions for `{word}`:\n{}",
        matches.join("\n")
    )))
}

fn find_references(file: &str, line: u64, col: u64, _ext: &str) -> Result<ToolResult, ToolError> {
    let word = word_at_position(file, line, col)?;
    let root = workspace_root_for(file);
    let pattern = regex::Regex::new(&format!(r"\b{}\b", regex::escape(&word)))
        .map_err(|e| ToolError::InvalidInput(format!("invalid regex: {e}")))?;
    let mut refs = Vec::new();
    let mut walker = ignore::WalkBuilder::new(root);
    walker.hidden(false).git_ignore(true).git_global(true);

    for entry in walker.build().flatten() {
        let path = entry.path();
        if !path.is_file() || crate::treesitter::SupportedLanguage::from_path(path).is_err() {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        for (index, line_text) in content.lines().enumerate() {
            if pattern.is_match(line_text) {
                refs.push(format!(
                    "{}:{}: {}",
                    path.display(),
                    index + 1,
                    line_text.trim()
                ));
                if refs.len() >= 200 {
                    break;
                }
            }
        }
        if refs.len() >= 200 {
            break;
        }
    }

    if refs.is_empty() {
        return Ok(ToolResult::success(format!(
            "No references found for `{word}` from {file}:{line}:{col}"
        )));
    }

    Ok(ToolResult::success(format!(
        "References for `{word}` ({} shown):\n{}",
        refs.len(),
        refs.join("\n")
    )))
}

fn hover_info(file: &str, line: u64, col: u64, _ext: &str) -> Result<ToolResult, ToolError> {
    let word = word_at_position(file, line, col)?;
    let path = Path::new(file);
    let line_index = usize::try_from(line)
        .map_err(|_| ToolError::InvalidInput(format!("line {line} is too large")))?;
    for symbol in parse_file_symbols(path) {
        let on_symbol =
            symbol.range.start_line <= line_index && symbol.range.end_line >= line_index;
        if symbol.name == word || on_symbol {
            let signature = symbol.signature.unwrap_or_else(|| symbol.name.clone());
            let doc = symbol.doc.unwrap_or_default();
            return Ok(ToolResult::success(format!(
                "{:?} `{}` at {}:{}-{}\n{}\n{}",
                symbol.kind,
                symbol.name,
                file,
                symbol.range.start_line,
                symbol.range.end_line,
                signature,
                doc
            )));
        }
    }

    Ok(ToolResult::success(format!(
        "No symbol metadata found for `{word}` at {file}:{line}:{col}"
    )))
}
