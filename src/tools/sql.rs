use cortex_sdk::{Tool, ToolCapabilities, ToolError, ToolResult};

/// Execute SQL queries against `SQLite` databases.
pub struct SqlTool;

impl Tool for SqlTool {
    fn name(&self) -> &'static str {
        "sql"
    }

    fn description(&self) -> &'static str {
        "Execute SQL queries against a SQLite database.\n\n\
         Use for inspecting data, debugging storage, or running analytical queries. \
         Read-only by default — set `write` to true for INSERT/UPDATE/DELETE."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "db": { "type": "string", "description": "Path to SQLite database file" },
                "query": { "type": "string", "description": "SQL query to execute" },
                "write": { "type": "boolean", "description": "Allow write operations (default: false)" }
            },
            "required": ["db", "query"]
        })
    }

    fn capabilities(&self) -> ToolCapabilities {
        super::caps([
            super::read_file_effect("SQLite database"),
            super::write_file_effect("SQLite database when write=true"),
        ])
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let db_path = input
            .get("db")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("missing 'db'".into()))?;
        let query = input
            .get("query")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("missing 'query'".into()))?;
        let allow_write = input
            .get("write")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        // Safety check
        let normalized = query.trim().to_uppercase();
        if !allow_write
            && (normalized.starts_with("INSERT")
                || normalized.starts_with("UPDATE")
                || normalized.starts_with("DELETE")
                || normalized.starts_with("DROP")
                || normalized.starts_with("ALTER")
                || normalized.starts_with("CREATE"))
        {
            return Err(ToolError::InvalidInput(
                "write operation blocked. Set write=true to allow.".into(),
            ));
        }

        let flags = if allow_write {
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
        } else {
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
        };

        let conn = rusqlite::Connection::open_with_flags(db_path, flags)
            .map_err(|e| ToolError::ExecutionFailed(format!("open {db_path}: {e}")))?;

        if normalized.starts_with("SELECT")
            || normalized.starts_with("PRAGMA")
            || normalized.starts_with("EXPLAIN")
            || normalized.starts_with("WITH")
        {
            // Query with results
            let mut stmt = conn
                .prepare(query)
                .map_err(|e| ToolError::ExecutionFailed(format!("prepare: {e}")))?;
            let col_count = stmt.column_count();
            let col_names: Vec<String> = (0..col_count)
                .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
                .collect();

            let mut rows_out = Vec::new();
            rows_out.push(col_names.join(" | "));
            rows_out.push("-".repeat(rows_out[0].len()));

            let mut rows = stmt
                .query([])
                .map_err(|e| ToolError::ExecutionFailed(format!("query: {e}")))?;

            let mut count = 0;
            while let Some(row) = rows
                .next()
                .map_err(|e| ToolError::ExecutionFailed(format!("row: {e}")))?
            {
                let vals: Vec<String> = (0..col_count)
                    .map(|i| {
                        row.get::<_, rusqlite::types::Value>(i).map_or_else(
                            |_| "?".into(),
                            |v| match v {
                                rusqlite::types::Value::Null => "NULL".into(),
                                rusqlite::types::Value::Integer(n) => n.to_string(),
                                rusqlite::types::Value::Real(f) => format!("{f:.4}"),
                                rusqlite::types::Value::Text(s) => {
                                    if s.len() > 100 {
                                        format!("{}...", &s[..100])
                                    } else {
                                        s
                                    }
                                }
                                rusqlite::types::Value::Blob(b) => format!("<blob {}B>", b.len()),
                            },
                        )
                    })
                    .collect();
                rows_out.push(vals.join(" | "));
                count += 1;
                if count >= 100 {
                    rows_out.push("... (truncated at 100 rows)".into());
                    break;
                }
            }

            Ok(ToolResult::success(format!(
                "{count} rows:\n{}",
                rows_out.join("\n")
            )))
        } else {
            // Execute without results
            let affected = conn
                .execute(query, [])
                .map_err(|e| ToolError::ExecutionFailed(format!("execute: {e}")))?;
            Ok(ToolResult::success(format!("{affected} rows affected")))
        }
    }
}
