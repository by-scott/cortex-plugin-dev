use cortex_sdk::{Tool, ToolError, ToolResult};

fn parse_cell_number(input: &serde_json::Value) -> Result<Option<usize>, ToolError> {
    input
        .get("cell_number")
        .and_then(serde_json::Value::as_u64)
        .map(|value| {
            usize::try_from(value).map_err(|_| {
                ToolError::InvalidInput(format!(
                    "'cell_number' is too large for this platform: {value}"
                ))
            })
        })
        .transpose()
}

pub struct NotebookEditTool;

impl Tool for NotebookEditTool {
    fn name(&self) -> &'static str {
        "notebook_edit"
    }

    fn description(&self) -> &'static str {
        "Edit a Jupyter notebook (.ipynb) cell.\n\n\
         Supports replace (overwrite cell source), insert (add new cell), and delete operations.\
         Cell numbers are 0-indexed."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to .ipynb file" },
                "cell_number": { "type": "integer", "description": "0-indexed cell number" },
                "new_source": { "type": "string", "description": "New cell source content" },
                "cell_type": { "type": "string", "enum": ["code", "markdown"], "description": "Cell type (for insert)" },
                "edit_mode": { "type": "string", "enum": ["replace", "insert", "delete"], "description": "Edit mode (default: replace)" }
            },
            "required": ["path", "new_source"]
        })
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let path = input
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("missing 'path'".into()))?;
        let new_source = input
            .get("new_source")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("missing 'new_source'".into()))?;
        let cell_num = parse_cell_number(&input)?;
        let edit_mode = input
            .get("edit_mode")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("replace");
        let cell_type = input
            .get("cell_type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("code");

        let content = std::fs::read_to_string(path)
            .map_err(|e| ToolError::ExecutionFailed(format!("cannot read {path}: {e}")))?;
        let mut nb: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| ToolError::ExecutionFailed(format!("invalid notebook JSON: {e}")))?;

        let cells = nb
            .get_mut("cells")
            .and_then(serde_json::Value::as_array_mut)
            .ok_or_else(|| ToolError::ExecutionFailed("notebook has no 'cells' array".into()))?;

        let source_lines: Vec<serde_json::Value> = new_source
            .lines()
            .map(|l| serde_json::Value::String(format!("{l}\n")))
            .collect();

        match edit_mode {
            "replace" => {
                let idx = cell_num.ok_or_else(|| {
                    ToolError::InvalidInput("'cell_number' required for replace".into())
                })?;
                if idx >= cells.len() {
                    return Err(ToolError::InvalidInput(format!(
                        "cell {idx} out of range (0..{})",
                        cells.len()
                    )));
                }
                cells[idx]["source"] = serde_json::Value::Array(source_lines);
                if let Some(ct) = input.get("cell_type").and_then(serde_json::Value::as_str) {
                    cells[idx]["cell_type"] = serde_json::Value::String(ct.into());
                }
            }
            "insert" => {
                let idx = cell_num.unwrap_or(cells.len());
                let new_cell = serde_json::json!({
                    "cell_type": cell_type,
                    "source": source_lines,
                    "metadata": {},
                    "outputs": []
                });
                if idx >= cells.len() {
                    cells.push(new_cell);
                } else {
                    cells.insert(idx, new_cell);
                }
            }
            "delete" => {
                let idx = cell_num.ok_or_else(|| {
                    ToolError::InvalidInput("'cell_number' required for delete".into())
                })?;
                if idx >= cells.len() {
                    return Err(ToolError::InvalidInput(format!("cell {idx} out of range")));
                }
                cells.remove(idx);
            }
            other => {
                return Err(ToolError::InvalidInput(format!(
                    "unknown edit_mode '{other}'"
                )));
            }
        }

        let output = serde_json::to_string_pretty(&nb)
            .map_err(|e| ToolError::ExecutionFailed(format!("serialize error: {e}")))?;
        std::fs::write(path, &output)
            .map_err(|e| ToolError::ExecutionFailed(format!("write error: {e}")))?;

        Ok(ToolResult::success(format!(
            "Notebook {path}: {edit_mode} at cell {}",
            cell_num.unwrap_or(0)
        )))
    }
}
