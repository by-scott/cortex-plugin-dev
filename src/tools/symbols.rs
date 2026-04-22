use cortex_sdk::{Tool, ToolError, ToolResult};
use std::path::{Path, PathBuf};

use crate::treesitter::{SupportedLanguage, Symbol, SymbolKind, Visibility};

#[derive(serde::Serialize)]
struct SymbolRecord {
    file: String,
    name: String,
    kind: String,
    start_line: usize,
    end_line: usize,
    visibility: String,
    parent: Option<String>,
    signature: Option<String>,
    doc: Option<String>,
}

fn parse_limit(input: &serde_json::Value, default: usize) -> Result<usize, ToolError> {
    input
        .get("limit")
        .and_then(serde_json::Value::as_u64)
        .map_or(Ok(default), |value| {
            usize::try_from(value)
                .map_err(|_| ToolError::InvalidInput(format!("limit is too large: {value}")))
        })
}

const fn kind_name(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Function => "function",
        SymbolKind::Method => "method",
        SymbolKind::Struct => "struct",
        SymbolKind::Class => "class",
        SymbolKind::Interface => "interface",
        SymbolKind::Enum => "enum",
        SymbolKind::Import => "import",
        SymbolKind::Constant => "constant",
        SymbolKind::Module => "module",
        SymbolKind::TypeAlias => "type_alias",
        SymbolKind::Trait => "trait",
        SymbolKind::Impl => "impl",
        SymbolKind::Macro => "macro",
        SymbolKind::Field => "field",
        SymbolKind::Variable => "variable",
        SymbolKind::Constructor => "constructor",
        SymbolKind::Property => "property",
    }
}

fn parse_kind(value: &str) -> Result<SymbolKind, ToolError> {
    match value {
        "function" => Ok(SymbolKind::Function),
        "method" => Ok(SymbolKind::Method),
        "struct" => Ok(SymbolKind::Struct),
        "class" => Ok(SymbolKind::Class),
        "interface" => Ok(SymbolKind::Interface),
        "enum" => Ok(SymbolKind::Enum),
        "import" => Ok(SymbolKind::Import),
        "constant" => Ok(SymbolKind::Constant),
        "module" => Ok(SymbolKind::Module),
        "type_alias" => Ok(SymbolKind::TypeAlias),
        "trait" => Ok(SymbolKind::Trait),
        "impl" => Ok(SymbolKind::Impl),
        "macro" => Ok(SymbolKind::Macro),
        "field" => Ok(SymbolKind::Field),
        "variable" => Ok(SymbolKind::Variable),
        "constructor" => Ok(SymbolKind::Constructor),
        "property" => Ok(SymbolKind::Property),
        other => Err(ToolError::InvalidInput(format!(
            "unknown symbol kind: {other}"
        ))),
    }
}

const fn visibility_name(visibility: Visibility) -> &'static str {
    match visibility {
        Visibility::Public => "public",
        Visibility::Private => "private",
    }
}

fn symbol_record(file: &str, symbol: &Symbol) -> SymbolRecord {
    SymbolRecord {
        file: file.to_string(),
        name: symbol.name.clone(),
        kind: kind_name(symbol.kind).to_string(),
        start_line: symbol.range.start_line,
        end_line: symbol.range.end_line,
        visibility: visibility_name(symbol.visibility).to_string(),
        parent: symbol.parent.clone(),
        signature: symbol.signature.clone(),
        doc: symbol.doc.clone(),
    }
}

fn format_symbol_line(symbol: &Symbol) -> String {
    let vis = match symbol.visibility {
        Visibility::Public => "pub ",
        Visibility::Private => "",
    };
    let parent = symbol
        .parent
        .as_ref()
        .map_or(String::new(), |name| format!("{name}."));
    let signature = symbol
        .signature
        .as_ref()
        .filter(|sig| !sig.is_empty())
        .map_or_else(|| symbol.name.clone(), Clone::clone);
    format!(
        "  L{}-{}: {vis}{} {parent}{signature}",
        symbol.range.start_line,
        symbol.range.end_line,
        kind_name(symbol.kind)
    )
}

fn filter_symbols<'a>(
    symbols: &'a [Symbol],
    kind_filter: Option<SymbolKind>,
    public_only: bool,
    query: Option<&str>,
) -> Vec<&'a Symbol> {
    let query = query.map(str::to_lowercase);
    symbols
        .iter()
        .filter(|symbol| {
            if public_only && symbol.visibility != Visibility::Public {
                return false;
            }
            if kind_filter.is_some_and(|kind| symbol.kind != kind) {
                return false;
            }
            if let Some(query) = &query {
                let name = symbol.name.to_lowercase();
                let parent = symbol.parent.as_deref().unwrap_or("").to_lowercase();
                return name.contains(query) || parent.contains(query);
            }
            true
        })
        .collect()
}

fn cache_path() -> PathBuf {
    std::env::var_os("CORTEX_DEV_SYMBOL_CACHE").map_or_else(
        || std::env::temp_dir().join("cortex-plugin-dev-symbols.sqlite"),
        PathBuf::from,
    )
}

fn read_cached_symbols(path: &Path) -> Result<Vec<Symbol>, ToolError> {
    let lang =
        SupportedLanguage::from_path(path).map_err(|e| ToolError::InvalidInput(format!("{e}")))?;
    let source = std::fs::read_to_string(path)
        .map_err(|e| ToolError::ExecutionFailed(format!("read error: {e}")))?;
    let cache = crate::symbol_cache::SymbolCache::open(cache_path())
        .map_err(|e| ToolError::ExecutionFailed(format!("{e}")))?;
    let entry = cache
        .get_or_parse(&path.to_string_lossy(), &source, lang)
        .map_err(|e| ToolError::ExecutionFailed(format!("{e}")))?;
    let crate::symbol_cache::CachedEntry { symbols, imports } = entry;
    let _import_count = imports.len();
    Ok(symbols)
}

fn is_supported_source(path: &Path) -> bool {
    SupportedLanguage::from_path(path).is_ok()
}

fn collect_workspace_symbols(
    base: &Path,
    kind_filter: Option<SymbolKind>,
    public_only: bool,
    query: Option<&str>,
    limit: usize,
) -> Result<Vec<SymbolRecord>, ToolError> {
    if !base.is_dir() {
        return Err(ToolError::InvalidInput(format!(
            "directory does not exist: {}",
            base.display()
        )));
    }

    let mut walker = ignore::WalkBuilder::new(base);
    walker.hidden(false).git_ignore(true).git_global(true);

    let mut records = Vec::new();
    for entry in walker.build().flatten() {
        let path = entry.path();
        if !path.is_file() || !is_supported_source(path) {
            continue;
        }
        let Ok(symbols) = read_cached_symbols(path) else {
            continue;
        };
        let file = path.to_string_lossy();
        for symbol in filter_symbols(&symbols, kind_filter, public_only, query) {
            records.push(symbol_record(&file, symbol));
            if records.len() >= limit {
                return Ok(records);
            }
        }
    }
    Ok(records)
}

/// Extract symbols (functions, structs, classes, traits, etc.) from source files.
/// Uses tree-sitter for accurate parsing — not regex heuristics.
pub struct SymbolsTool;

impl Tool for SymbolsTool {
    fn name(&self) -> &'static str {
        "symbols"
    }

    fn description(&self) -> &'static str {
        "Extract code symbols from a source file using tree-sitter parsing.\n\n\
         Returns functions, methods, structs, classes, interfaces, traits, impls, enums, \
         constants, variables, properties, macros, modules, imports, line ranges, visibility, \
         signatures, parent symbols, and doc comments. Much more accurate than grep for code navigation.\n\n\
         Supported languages: Rust (.rs), Python (.py), TypeScript (.ts), TSX (.tsx)."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to source file"
                },
                "kind": {
                    "type": "string",
                    "description": "Filter by symbol kind (default: all)",
                    "enum": ["function", "method", "struct", "class", "interface", "enum", "import", "constant", "module", "type_alias", "trait", "impl", "macro", "field", "variable", "constructor", "property"]
                },
                "public_only": {
                    "type": "boolean",
                    "description": "Only show public/exported symbols (default: false)"
                },
                "format": {
                    "type": "string",
                    "description": "Output format: text or json (default: text)",
                    "enum": ["text", "json"]
                }
            },
            "required": ["path"]
        })
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let path_str = input
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("missing 'path'".into()))?;

        let path = std::path::Path::new(path_str);
        if !path.exists() {
            return Err(ToolError::InvalidInput(format!(
                "file not found: {path_str}"
            )));
        }

        let symbols = read_cached_symbols(path)?;
        let kind_filter = input
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .map(parse_kind)
            .transpose()?;
        let public_only = input
            .get("public_only")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let format = input
            .get("format")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("text");

        let filtered = filter_symbols(&symbols, kind_filter, public_only, None);

        if filtered.is_empty() {
            return Ok(ToolResult::success(format!(
                "No symbols found in {path_str}"
            )));
        }

        if format == "json" {
            let records: Vec<_> = filtered
                .iter()
                .map(|symbol| symbol_record(path_str, symbol))
                .collect();
            let json = serde_json::to_string_pretty(&records)
                .map_err(|e| ToolError::ExecutionFailed(format!("json error: {e}")))?;
            return Ok(ToolResult::success(json));
        }

        let mut lines = Vec::with_capacity(filtered.len());
        for s in &filtered {
            lines.push(format_symbol_line(s));
        }

        Ok(ToolResult::success(format!(
            "{path_str} ({} symbols):\n{}",
            filtered.len(),
            lines.join("\n")
        )))
    }
}

/// Build or query a workspace-level symbol index.
pub struct SymbolIndexTool;

impl Tool for SymbolIndexTool {
    fn name(&self) -> &'static str {
        "symbol_index"
    }

    fn description(&self) -> &'static str {
        "Index code symbols across a workspace.\n\n\
         Scans Rust, Python, TypeScript, and TSX files with tree-sitter, respects gitignore, \
         uses a content-hash SQLite cache, and returns structured navigation records. \
         Use before broad refactors, architectural mapping, or locating definitions across a project."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Workspace directory (default: current directory)" },
                "query": { "type": "string", "description": "Optional case-insensitive symbol or parent-name filter" },
                "kind": {
                    "type": "string",
                    "description": "Optional symbol kind filter",
                    "enum": ["function", "method", "struct", "class", "interface", "enum", "import", "constant", "module", "type_alias", "trait", "impl", "macro", "field", "variable", "constructor", "property"]
                },
                "public_only": { "type": "boolean", "description": "Only public/exported symbols (default: false)" },
                "limit": { "type": "integer", "description": "Maximum records (default: 500)" },
                "format": {
                    "type": "string",
                    "description": "Output format: text or json (default: text)",
                    "enum": ["text", "json"]
                }
            }
        })
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let base_path = input
            .get("path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(".");
        let query = input.get("query").and_then(serde_json::Value::as_str);
        let kind_filter = input
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .map(parse_kind)
            .transpose()?;
        let public_only = input
            .get("public_only")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let limit = parse_limit(&input, 500)?;
        let format = input
            .get("format")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("text");

        let records = collect_workspace_symbols(
            Path::new(base_path),
            kind_filter,
            public_only,
            query,
            limit,
        )?;

        if records.is_empty() {
            return Ok(ToolResult::success(format!(
                "No indexed symbols found in {base_path}"
            )));
        }

        if format == "json" {
            let json = serde_json::to_string_pretty(&records)
                .map_err(|e| ToolError::ExecutionFailed(format!("json error: {e}")))?;
            return Ok(ToolResult::success(json));
        }

        let mut lines = Vec::with_capacity(records.len() + 1);
        lines.push(format!(
            "{base_path} indexed symbols ({} shown, cache: {}):",
            records.len(),
            cache_path().display()
        ));
        for record in &records {
            let parent = record
                .parent
                .as_ref()
                .map_or(String::new(), |name| format!("{name}."));
            let name = record.signature.as_ref().unwrap_or(&record.name);
            lines.push(format!(
                "  {}:{}-{}: {} {}{}",
                record.file, record.start_line, record.end_line, record.kind, parent, name
            ));
        }

        Ok(ToolResult::success(lines.join("\n")))
    }
}

/// Search symbols across a workspace.
pub struct SymbolSearchTool;

impl Tool for SymbolSearchTool {
    fn name(&self) -> &'static str {
        "symbol_search"
    }

    fn description(&self) -> &'static str {
        "Search workspace symbols by name, kind, visibility, or parent.\n\n\
         Uses the same tree-sitter index and content-hash cache as `symbol_index`, but optimized \
         for targeted lookup. Use it before grep when searching definitions rather than arbitrary text."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Case-insensitive symbol or parent-name query" },
                "path": { "type": "string", "description": "Workspace directory (default: current directory)" },
                "kind": {
                    "type": "string",
                    "description": "Optional symbol kind filter",
                    "enum": ["function", "method", "struct", "class", "interface", "enum", "import", "constant", "module", "type_alias", "trait", "impl", "macro", "field", "variable", "constructor", "property"]
                },
                "public_only": { "type": "boolean", "description": "Only public/exported symbols (default: false)" },
                "limit": { "type": "integer", "description": "Maximum records (default: 100)" },
                "format": {
                    "type": "string",
                    "description": "Output format: text or json (default: text)",
                    "enum": ["text", "json"]
                }
            },
            "required": ["query"]
        })
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let query = input
            .get("query")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("missing 'query'".into()))?;
        let base_path = input
            .get("path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(".");
        let kind_filter = input
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .map(parse_kind)
            .transpose()?;
        let public_only = input
            .get("public_only")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let limit = parse_limit(&input, 100)?;
        let format = input
            .get("format")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("text");

        let records = collect_workspace_symbols(
            Path::new(base_path),
            kind_filter,
            public_only,
            Some(query),
            limit,
        )?;

        if records.is_empty() {
            return Ok(ToolResult::success(format!(
                "No symbols matching '{query}' in {base_path}"
            )));
        }

        if format == "json" {
            let json = serde_json::to_string_pretty(&records)
                .map_err(|e| ToolError::ExecutionFailed(format!("json error: {e}")))?;
            return Ok(ToolResult::success(json));
        }

        let mut lines = Vec::with_capacity(records.len() + 1);
        lines.push(format!("Symbols matching '{query}' ({}):", records.len()));
        for record in &records {
            let parent = record
                .parent
                .as_ref()
                .map_or(String::new(), |name| format!("{name}."));
            let name = record.signature.as_ref().unwrap_or(&record.name);
            lines.push(format!(
                "  {}:{}-{}: {} {}{}",
                record.file, record.start_line, record.end_line, record.kind, parent, name
            ));
        }

        Ok(ToolResult::success(lines.join("\n")))
    }
}

/// Extract import/dependency edges from a source file.
pub struct ImportsTool;

impl Tool for ImportsTool {
    fn name(&self) -> &'static str {
        "imports"
    }

    fn description(&self) -> &'static str {
        "Extract import/dependency relationships from a source file.\n\n\
         Shows what the file depends on — use modules, Python imports, TypeScript imports. \
         Useful for understanding dependency structure without reading the full file."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to source file"
                }
            },
            "required": ["path"]
        })
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let path_str = input
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("missing 'path'".into()))?;

        let path = std::path::Path::new(path_str);
        if !path.exists() {
            return Err(ToolError::InvalidInput(format!(
                "file not found: {path_str}"
            )));
        }

        let lang = crate::treesitter::SupportedLanguage::from_path(path)
            .map_err(|e| ToolError::InvalidInput(format!("{e}")))?;

        let source = std::fs::read_to_string(path)
            .map_err(|e| ToolError::ExecutionFailed(format!("read error: {e}")))?;

        let imports = crate::treesitter::extract_imports(&source, lang, path_str)
            .map_err(|e| ToolError::ExecutionFailed(format!("parse error: {e}")))?;

        if imports.is_empty() {
            return Ok(ToolResult::success(format!("No imports in {path_str}")));
        }

        let mut lines = Vec::with_capacity(imports.len());
        for imp in &imports {
            if let Some(ref alias) = imp.alias {
                lines.push(format!("  {} (as {alias})", imp.imported_path));
            } else {
                lines.push(format!("  {}", imp.imported_path));
            }
        }

        Ok(ToolResult::success(format!(
            "{path_str} imports ({}):\n{}",
            imports.len(),
            lines.join("\n")
        )))
    }
}
