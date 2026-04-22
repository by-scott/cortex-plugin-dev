use std::fmt;
use std::path::Path;

use streaming_iterator::StreamingIterator;

// -- Error type --

#[derive(Debug)]
pub enum TreeSitterError {
    UnsupportedLanguage(String),
    ParseFailed(String),
    IoError(std::io::Error),
    QueryError(String),
}

impl fmt::Display for TreeSitterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedLanguage(ext) => write!(f, "unsupported language: {ext}"),
            Self::ParseFailed(msg) => write!(f, "parse failed: {msg}"),
            Self::IoError(e) => write!(f, "IO error: {e}"),
            Self::QueryError(msg) => write!(f, "query error: {msg}"),
        }
    }
}

impl std::error::Error for TreeSitterError {}

impl From<std::io::Error> for TreeSitterError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

// -- Supported languages --

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupportedLanguage {
    Rust,
    Python,
    TypeScript,
    Tsx,
}

impl SupportedLanguage {
    /// Resolve a language from a file extension.
    ///
    /// # Errors
    ///
    /// Returns `TreeSitterError::UnsupportedLanguage` if the extension is not
    /// recognized.
    pub fn from_extension(ext: &str) -> Result<Self, TreeSitterError> {
        match ext {
            "rs" => Ok(Self::Rust),
            "py" => Ok(Self::Python),
            "ts" => Ok(Self::TypeScript),
            "tsx" => Ok(Self::Tsx),
            other => Err(TreeSitterError::UnsupportedLanguage(other.to_string())),
        }
    }

    /// Resolve a language from a file path's extension.
    ///
    /// # Errors
    ///
    /// Returns `TreeSitterError::UnsupportedLanguage` if the file has no
    /// recognized extension.
    pub fn from_path(path: &Path) -> Result<Self, TreeSitterError> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| TreeSitterError::UnsupportedLanguage("no extension".into()))?;
        Self::from_extension(ext)
    }

    fn ts_language(self) -> tree_sitter::Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        }
    }
}

// -- Symbol types --

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SymbolRange {
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Visibility {
    Public,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SymbolKind {
    Function,
    Method,
    Struct,
    Class,
    Interface,
    Enum,
    Import,
    Constant,
    Module,
    TypeAlias,
    Trait,
    Impl,
    Macro,
    Field,
    Variable,
    Constructor,
    Property,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub range: SymbolRange,
    pub visibility: Visibility,
    pub doc: Option<String>,
    pub signature: Option<String>,
    pub parent: Option<String>,
}

// -- Import edge --

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ImportEdge {
    pub source_file: String,
    pub imported_path: String,
    pub alias: Option<String>,
}

// -- CodeParser --

pub struct CodeParser {
    parser: tree_sitter::Parser,
}

impl CodeParser {
    #[must_use]
    pub fn new() -> Self {
        Self {
            parser: tree_sitter::Parser::new(),
        }
    }

    /// Parse a source file into a tree-sitter syntax tree.
    ///
    /// # Errors
    ///
    /// Returns `TreeSitterError` if the language is unsupported, the file
    /// cannot be read, or parsing fails.
    pub fn parse_file(&mut self, path: &Path) -> Result<tree_sitter::Tree, TreeSitterError> {
        let lang = SupportedLanguage::from_path(path)?;
        let source = std::fs::read_to_string(path)?;
        self.parse_str(&source, lang)
    }

    /// Parse a source string into a tree-sitter syntax tree.
    ///
    /// # Errors
    ///
    /// Returns `TreeSitterError` if language setup or parsing fails.
    pub fn parse_str(
        &mut self,
        source: &str,
        lang: SupportedLanguage,
    ) -> Result<tree_sitter::Tree, TreeSitterError> {
        self.parser
            .set_language(&lang.ts_language())
            .map_err(|e| TreeSitterError::ParseFailed(format!("set_language: {e}")))?;
        self.parser
            .parse(source, None)
            .ok_or_else(|| TreeSitterError::ParseFailed("parser returned None".into()))
    }
}

impl Default for CodeParser {
    fn default() -> Self {
        Self::new()
    }
}

// -- Helpers --

fn node_text<'a>(node: &tree_sitter::Node, source: &'a str) -> &'a str {
    &source[node.start_byte()..node.end_byte()]
}

fn has_pub_modifier(node: &tree_sitter::Node, source: &str) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "visibility_modifier" {
            let text = node_text(&child, source);
            if text.starts_with("pub") {
                return true;
            }
        }
    }
    false
}

fn has_export_keyword(node: &tree_sitter::Node) -> bool {
    node.parent()
        .is_some_and(|parent| parent.kind() == "export_statement")
}

/// Snapshot of one query match: which captures were found and their node info.
struct MatchSnapshot {
    captures: Vec<(String, CapturedNode)>,
}

struct CapturedNode {
    text: String,
    start_row: usize,
    end_row: usize,
    is_pub: bool,
    is_export: bool,
    parent_name: Option<String>,
}

fn collect_matches(
    query: &tree_sitter::Query,
    cursor: &mut tree_sitter::QueryCursor,
    root: tree_sitter::Node,
    source: &str,
) -> Vec<MatchSnapshot> {
    let mut results = Vec::new();
    let mut matches = cursor.matches(query, root, source.as_bytes());
    while let Some(m) = matches.next() {
        let mut captures = Vec::new();
        for cap in m.captures {
            let name = query.capture_names()[cap.index as usize].to_string();
            captures.push((
                name,
                CapturedNode {
                    text: node_text(&cap.node, source).to_string(),
                    start_row: cap.node.start_position().row,
                    end_row: cap.node.end_position().row,
                    is_pub: has_pub_modifier(&cap.node, source),
                    is_export: has_export_keyword(&cap.node),
                    parent_name: enclosing_symbol_name(cap.node, source),
                },
            ));
        }
        results.push(MatchSnapshot { captures });
    }
    results
}

impl MatchSnapshot {
    fn get(&self, name: &str) -> Option<&CapturedNode> {
        self.captures
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, c)| c)
    }
}

fn enclosing_symbol_name(node: tree_sitter::Node, source: &str) -> Option<String> {
    let mut current = node.parent();
    while let Some(parent) = current {
        if matches!(
            parent.kind(),
            "class_definition" | "class_declaration" | "impl_item" | "trait_item"
        ) {
            return first_named_child_text(parent, source);
        }
        current = parent.parent();
    }
    None
}

fn first_named_child_text(node: tree_sitter::Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| {
            matches!(
                child.kind(),
                "identifier" | "type_identifier" | "property_identifier"
            )
        })
        .map(|child| node_text(&child, source).to_string())
}

fn doc_before(start_row: usize, source: &str) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();
    let mut docs = Vec::new();
    let mut row = start_row;
    while row > 0 {
        row -= 1;
        let line = lines.get(row)?.trim();
        let text = line
            .strip_prefix("///")
            .or_else(|| line.strip_prefix("//!"))
            .or_else(|| line.strip_prefix("###"))
            .or_else(|| line.strip_prefix("##"))
            .or_else(|| line.strip_prefix('#'))
            .or_else(|| line.strip_prefix("/**"))
            .or_else(|| line.strip_prefix('*'))
            .map(str::trim)
            .map(|s| s.trim_end_matches("*/").trim());
        let Some(text) = text else {
            if line.is_empty() {
                continue;
            }
            break;
        };
        docs.push(text.to_string());
    }
    docs.reverse();
    (!docs.is_empty()).then(|| docs.join("\n"))
}

fn signature_from(def_node: &CapturedNode, name: &str) -> Option<String> {
    let first = def_node.text.lines().next()?.trim();
    if first.is_empty() {
        return None;
    }
    let collapsed = first.split_whitespace().collect::<Vec<_>>().join(" ");
    let signature = collapsed
        .split('{')
        .next()
        .unwrap_or(&collapsed)
        .trim_end_matches(':')
        .trim()
        .to_string();
    if signature.contains(name) {
        Some(signature)
    } else {
        None
    }
}

fn make_symbol(
    name_node: &CapturedNode,
    def_node: &CapturedNode,
    kind: SymbolKind,
    source: &str,
) -> Symbol {
    Symbol {
        name: name_node.text.clone(),
        kind,
        range: SymbolRange {
            start_line: def_node.start_row + 1,
            end_line: def_node.end_row + 1,
        },
        visibility: if def_node.is_pub {
            Visibility::Public
        } else {
            Visibility::Private
        },
        doc: doc_before(def_node.start_row, source),
        signature: signature_from(def_node, &name_node.text),
        parent: def_node.parent_name.clone(),
    }
}

fn make_symbol_exported(
    name_node: &CapturedNode,
    def_node: &CapturedNode,
    kind: SymbolKind,
    source: &str,
) -> Symbol {
    Symbol {
        name: name_node.text.clone(),
        kind,
        range: SymbolRange {
            start_line: def_node.start_row + 1,
            end_line: def_node.end_row + 1,
        },
        visibility: if def_node.is_export {
            Visibility::Public
        } else {
            Visibility::Private
        },
        doc: doc_before(def_node.start_row, source),
        signature: signature_from(def_node, &name_node.text),
        parent: def_node.parent_name.clone(),
    }
}

fn make_python_symbol(
    name_node: &CapturedNode,
    def_node: &CapturedNode,
    kind: SymbolKind,
    source: &str,
) -> Symbol {
    let is_private = name_node.text.starts_with('_');
    Symbol {
        name: name_node.text.clone(),
        kind,
        range: SymbolRange {
            start_line: def_node.start_row + 1,
            end_line: def_node.end_row + 1,
        },
        visibility: if is_private {
            Visibility::Private
        } else {
            Visibility::Public
        },
        doc: doc_before(def_node.start_row, source),
        signature: signature_from(def_node, &name_node.text),
        parent: def_node.parent_name.clone(),
    }
}

fn make_statement_symbol(
    stmt: &CapturedNode,
    name: String,
    kind: SymbolKind,
    source: &str,
) -> Symbol {
    Symbol {
        name,
        kind,
        range: SymbolRange {
            start_line: stmt.start_row + 1,
            end_line: stmt.end_row + 1,
        },
        visibility: Visibility::Private,
        doc: doc_before(stmt.start_row, source),
        signature: Some(stmt.text.clone()),
        parent: stmt.parent_name.clone(),
    }
}

// -- Symbol extraction queries --

const fn rust_symbol_query() -> &'static str {
    r"
    (function_item
      name: (identifier) @fn_name) @fn_def

    (impl_item
      body: (declaration_list
        (function_item
          name: (identifier) @method_name) @method_def))

    (impl_item
      trait: (type_identifier)? @impl_trait
      type: (type_identifier) @impl_type) @impl_def

    (struct_item
      name: (type_identifier) @struct_name) @struct_def

    (enum_item
      name: (type_identifier) @enum_name) @enum_def

    (trait_item
      name: (type_identifier) @trait_name) @trait_def

    (macro_definition
      name: (identifier) @macro_name) @macro_def

    (const_item
      name: (identifier) @const_name) @const_def

    (static_item
      name: (identifier) @static_name) @static_def

    (type_item
      name: (type_identifier) @type_name) @type_def

    (use_declaration) @use_decl

    (mod_item
      name: (identifier) @mod_name) @mod_def
    "
}

const fn python_symbol_query() -> &'static str {
    r"
    (function_definition
      name: (identifier) @fn_name) @fn_def

    (class_definition
      name: (identifier) @class_name) @class_def

    (assignment
      left: (identifier) @var_name) @var_def

    (import_statement) @import_stmt

    (import_from_statement) @import_from_stmt
    "
}

const fn typescript_symbol_query() -> &'static str {
    r"
    (function_declaration
      name: (identifier) @fn_name) @fn_def

    (class_declaration
      name: (type_identifier) @class_name) @class_def

    (method_definition
      name: [(property_identifier) (identifier)] @method_name) @method_def

    (interface_declaration
      name: (type_identifier) @iface_name) @iface_def

    (enum_declaration
      name: (identifier) @enum_name) @enum_def

    (type_alias_declaration
      name: (type_identifier) @type_name) @type_def

    (lexical_declaration
      (variable_declarator
        name: (identifier) @var_name)) @var_def

    (public_field_definition
      name: (property_identifier) @field_name) @field_def

    (import_statement) @import_stmt
    "
}

// -- Rust symbol extraction --

fn extract_rust_symbols(
    tree: &tree_sitter::Tree,
    source: &str,
) -> Result<Vec<Symbol>, TreeSitterError> {
    let lang: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
    let query = tree_sitter::Query::new(&lang, rust_symbol_query())
        .map_err(|e| TreeSitterError::QueryError(format!("{e}")))?;
    let mut cursor = tree_sitter::QueryCursor::new();
    let snapshots = collect_matches(&query, &mut cursor, tree.root_node(), source);

    let mut symbols = Vec::new();
    for snap in &snapshots {
        if let (Some(name), Some(def)) = (snap.get("fn_name"), snap.get("fn_def")) {
            symbols.push(make_symbol(name, def, SymbolKind::Function, source));
        } else if let (Some(name), Some(def)) = (snap.get("method_name"), snap.get("method_def")) {
            symbols.push(make_symbol(name, def, SymbolKind::Method, source));
        } else if let (Some(name), Some(def)) = (snap.get("impl_type"), snap.get("impl_def")) {
            symbols.push(make_symbol(name, def, SymbolKind::Impl, source));
        } else if let (Some(name), Some(def)) = (snap.get("struct_name"), snap.get("struct_def")) {
            symbols.push(make_symbol(name, def, SymbolKind::Struct, source));
        } else if let (Some(name), Some(def)) = (snap.get("enum_name"), snap.get("enum_def")) {
            symbols.push(make_symbol(name, def, SymbolKind::Enum, source));
        } else if let (Some(name), Some(def)) = (snap.get("trait_name"), snap.get("trait_def")) {
            symbols.push(make_symbol(name, def, SymbolKind::Trait, source));
        } else if let (Some(name), Some(def)) = (snap.get("macro_name"), snap.get("macro_def")) {
            symbols.push(make_symbol(name, def, SymbolKind::Macro, source));
        } else if let (Some(name), Some(def)) = (snap.get("const_name"), snap.get("const_def")) {
            symbols.push(make_symbol(name, def, SymbolKind::Constant, source));
        } else if let (Some(name), Some(def)) = (snap.get("static_name"), snap.get("static_def")) {
            symbols.push(make_symbol(name, def, SymbolKind::Constant, source));
        } else if let (Some(name), Some(def)) = (snap.get("type_name"), snap.get("type_def")) {
            symbols.push(make_symbol(name, def, SymbolKind::TypeAlias, source));
        } else if let Some(decl) = snap.get("use_decl") {
            let name = decl
                .text
                .trim_start_matches("pub ")
                .trim_start_matches("use ")
                .trim_end_matches(';')
                .to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Import,
                range: SymbolRange {
                    start_line: decl.start_row + 1,
                    end_line: decl.end_row + 1,
                },
                visibility: if decl.is_pub {
                    Visibility::Public
                } else {
                    Visibility::Private
                },
                doc: doc_before(decl.start_row, source),
                signature: Some(decl.text.clone()),
                parent: decl.parent_name.clone(),
            });
        } else if let (Some(name), Some(def)) = (snap.get("mod_name"), snap.get("mod_def")) {
            symbols.push(make_symbol(name, def, SymbolKind::Module, source));
        }
    }
    Ok(symbols)
}

// -- Python symbol extraction --

fn extract_python_symbols(
    tree: &tree_sitter::Tree,
    source: &str,
) -> Result<Vec<Symbol>, TreeSitterError> {
    let lang: tree_sitter::Language = tree_sitter_python::LANGUAGE.into();
    let query = tree_sitter::Query::new(&lang, python_symbol_query())
        .map_err(|e| TreeSitterError::QueryError(format!("{e}")))?;
    let mut cursor = tree_sitter::QueryCursor::new();
    let snapshots = collect_matches(&query, &mut cursor, tree.root_node(), source);

    let mut symbols = Vec::new();
    for snap in &snapshots {
        if let (Some(name_node), Some(def_node)) = (snap.get("fn_name"), snap.get("fn_def")) {
            let kind = if def_node.parent_name.is_some() {
                SymbolKind::Method
            } else {
                SymbolKind::Function
            };
            symbols.push(make_python_symbol(name_node, def_node, kind, source));
        } else if let (Some(name_node), Some(def_node)) =
            (snap.get("class_name"), snap.get("class_def"))
        {
            symbols.push(make_python_symbol(
                name_node,
                def_node,
                SymbolKind::Class,
                source,
            ));
        } else if let (Some(name_node), Some(def_node)) =
            (snap.get("var_name"), snap.get("var_def"))
        {
            if !name_node.text.starts_with('_')
                && name_node
                    .text
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c == '_')
            {
                symbols.push(make_python_symbol(
                    name_node,
                    def_node,
                    SymbolKind::Constant,
                    source,
                ));
            }
        } else if let Some(stmt) = snap.get("import_stmt") {
            let name = stmt.text.trim_start_matches("import ").trim().to_string();
            symbols.push(make_statement_symbol(
                stmt,
                name,
                SymbolKind::Import,
                source,
            ));
        } else if let Some(stmt) = snap.get("import_from_stmt") {
            symbols.push(make_statement_symbol(
                stmt,
                stmt.text.clone(),
                SymbolKind::Import,
                source,
            ));
        }
    }
    Ok(symbols)
}

// -- TypeScript symbol extraction --

fn extract_typescript_symbols(
    tree: &tree_sitter::Tree,
    source: &str,
    lang: SupportedLanguage,
) -> Result<Vec<Symbol>, TreeSitterError> {
    let ts_lang = lang.ts_language();
    let query = tree_sitter::Query::new(&ts_lang, typescript_symbol_query())
        .map_err(|e| TreeSitterError::QueryError(format!("{e}")))?;
    let mut cursor = tree_sitter::QueryCursor::new();
    let snapshots = collect_matches(&query, &mut cursor, tree.root_node(), source);

    let mut symbols = Vec::new();
    for snap in &snapshots {
        if let (Some(name), Some(def)) = (snap.get("fn_name"), snap.get("fn_def")) {
            symbols.push(make_symbol_exported(
                name,
                def,
                SymbolKind::Function,
                source,
            ));
        } else if let (Some(name), Some(def)) = (snap.get("class_name"), snap.get("class_def")) {
            symbols.push(make_symbol_exported(name, def, SymbolKind::Class, source));
        } else if let (Some(name), Some(def)) = (snap.get("method_name"), snap.get("method_def")) {
            symbols.push(make_symbol_exported(name, def, SymbolKind::Method, source));
        } else if let (Some(name), Some(def)) = (snap.get("iface_name"), snap.get("iface_def")) {
            symbols.push(make_symbol_exported(
                name,
                def,
                SymbolKind::Interface,
                source,
            ));
        } else if let (Some(name), Some(def)) = (snap.get("enum_name"), snap.get("enum_def")) {
            symbols.push(make_symbol_exported(name, def, SymbolKind::Enum, source));
        } else if let (Some(name), Some(def)) = (snap.get("type_name"), snap.get("type_def")) {
            symbols.push(make_symbol_exported(
                name,
                def,
                SymbolKind::TypeAlias,
                source,
            ));
        } else if let (Some(name), Some(def)) = (snap.get("var_name"), snap.get("var_def")) {
            let kind = if name
                .text
                .chars()
                .all(|c| c.is_ascii_uppercase() || c == '_')
            {
                SymbolKind::Constant
            } else {
                SymbolKind::Variable
            };
            symbols.push(make_symbol_exported(name, def, kind, source));
        } else if let (Some(name), Some(def)) = (snap.get("field_name"), snap.get("field_def")) {
            symbols.push(make_symbol_exported(
                name,
                def,
                SymbolKind::Property,
                source,
            ));
        } else if let Some(stmt) = snap.get("import_stmt") {
            symbols.push(Symbol {
                name: stmt.text.clone(),
                kind: SymbolKind::Import,
                range: SymbolRange {
                    start_line: stmt.start_row + 1,
                    end_line: stmt.end_row + 1,
                },
                visibility: Visibility::Private,
                doc: doc_before(stmt.start_row, source),
                signature: Some(stmt.text.clone()),
                parent: stmt.parent_name.clone(),
            });
        }
    }
    Ok(symbols)
}

// -- Import extraction --

fn extract_rust_imports(
    tree: &tree_sitter::Tree,
    source: &str,
    source_file: &str,
) -> Result<Vec<ImportEdge>, TreeSitterError> {
    let lang: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
    let query = tree_sitter::Query::new(&lang, "(use_declaration) @use_decl")
        .map_err(|e| TreeSitterError::QueryError(format!("{e}")))?;
    let mut cursor = tree_sitter::QueryCursor::new();
    let snapshots = collect_matches(&query, &mut cursor, tree.root_node(), source);

    let mut edges = Vec::new();
    for snap in &snapshots {
        let Some(decl) = snap.get("use_decl") else {
            continue;
        };
        let inner = decl
            .text
            .trim_start_matches("pub ")
            .trim_start_matches("use ")
            .trim_end_matches(';')
            .trim();

        if let Some(as_pos) = inner.rfind(" as ") {
            let path = inner[..as_pos].trim();
            let alias = inner[as_pos + 4..].trim();
            edges.push(ImportEdge {
                source_file: source_file.to_string(),
                imported_path: path.to_string(),
                alias: Some(alias.to_string()),
            });
        } else if let Some(brace_start) = inner.find('{') {
            let prefix = &inner[..brace_start];
            let group = inner[brace_start + 1..].trim_end_matches('}').trim();
            for item in group.split(',') {
                let item = item.trim();
                if item.is_empty() {
                    continue;
                }
                if let Some(as_pos) = item.find(" as ") {
                    let name = item[..as_pos].trim();
                    let alias = item[as_pos + 4..].trim();
                    edges.push(ImportEdge {
                        source_file: source_file.to_string(),
                        imported_path: format!("{prefix}{name}"),
                        alias: Some(alias.to_string()),
                    });
                } else {
                    edges.push(ImportEdge {
                        source_file: source_file.to_string(),
                        imported_path: format!("{prefix}{item}"),
                        alias: None,
                    });
                }
            }
        } else {
            edges.push(ImportEdge {
                source_file: source_file.to_string(),
                imported_path: inner.to_string(),
                alias: None,
            });
        }
    }
    Ok(edges)
}

fn extract_python_imports(
    tree: &tree_sitter::Tree,
    source: &str,
    source_file: &str,
) -> Result<Vec<ImportEdge>, TreeSitterError> {
    let lang: tree_sitter::Language = tree_sitter_python::LANGUAGE.into();
    let query_str = r"
        (import_statement) @import_stmt
        (import_from_statement) @import_from_stmt
    ";
    let query = tree_sitter::Query::new(&lang, query_str)
        .map_err(|e| TreeSitterError::QueryError(format!("{e}")))?;
    let mut cursor = tree_sitter::QueryCursor::new();
    let snapshots = collect_matches(&query, &mut cursor, tree.root_node(), source);

    let mut edges = Vec::new();
    for snap in &snapshots {
        if let Some(stmt) = snap.get("import_stmt") {
            let rest = stmt.text.trim_start_matches("import ").trim();
            for item in rest.split(',') {
                let item = item.trim();
                if let Some(as_pos) = item.find(" as ") {
                    let path = item[..as_pos].trim();
                    let alias = item[as_pos + 4..].trim();
                    edges.push(ImportEdge {
                        source_file: source_file.to_string(),
                        imported_path: path.to_string(),
                        alias: Some(alias.to_string()),
                    });
                } else {
                    edges.push(ImportEdge {
                        source_file: source_file.to_string(),
                        imported_path: item.to_string(),
                        alias: None,
                    });
                }
            }
        } else if let Some(stmt) = snap.get("import_from_stmt") {
            let rest = stmt.text.trim_start_matches("from ").trim();
            if let Some(import_pos) = rest.find(" import ") {
                let module = rest[..import_pos].trim();
                let names = rest[import_pos + 8..].trim();
                for item in names.split(',') {
                    let item = item.trim();
                    if let Some(as_pos) = item.find(" as ") {
                        let name = item[..as_pos].trim();
                        let alias = item[as_pos + 4..].trim();
                        edges.push(ImportEdge {
                            source_file: source_file.to_string(),
                            imported_path: format!("{module}.{name}"),
                            alias: Some(alias.to_string()),
                        });
                    } else {
                        edges.push(ImportEdge {
                            source_file: source_file.to_string(),
                            imported_path: format!("{module}.{item}"),
                            alias: None,
                        });
                    }
                }
            }
        }
    }
    Ok(edges)
}

fn extract_typescript_imports(
    tree: &tree_sitter::Tree,
    source: &str,
    source_file: &str,
    lang: SupportedLanguage,
) -> Result<Vec<ImportEdge>, TreeSitterError> {
    let ts_lang = lang.ts_language();
    let query = tree_sitter::Query::new(&ts_lang, "(import_statement) @import_stmt")
        .map_err(|e| TreeSitterError::QueryError(format!("{e}")))?;
    let mut cursor = tree_sitter::QueryCursor::new();
    let snapshots = collect_matches(&query, &mut cursor, tree.root_node(), source);

    let mut edges = Vec::new();
    for snap in &snapshots {
        let Some(stmt) = snap.get("import_stmt") else {
            continue;
        };
        let text = &stmt.text;
        let module = text.rfind("from").map_or_else(
            || {
                text.find(['\'', '"']).map_or_else(
                    || text.clone(),
                    |quote_start| {
                        let quote_char = text.as_bytes()[quote_start] as char;
                        text[quote_start + 1..].find(quote_char).map_or_else(
                            || text.clone(),
                            |quote_end| {
                                text[quote_start + 1..quote_start + 1 + quote_end].to_string()
                            },
                        )
                    },
                )
            },
            |from_pos| {
                let after_from = text[from_pos + 4..].trim();
                after_from
                    .trim_matches(|c| c == '\'' || c == '"' || c == ';' || c == ' ')
                    .to_string()
            },
        );

        let alias = if text.contains("* as ") {
            text.split("* as ")
                .nth(1)
                .and_then(|s| s.split_whitespace().next())
                .map(std::string::ToString::to_string)
        } else {
            None
        };

        edges.push(ImportEdge {
            source_file: source_file.to_string(),
            imported_path: module,
            alias,
        });
    }
    Ok(edges)
}

// -- Public top-level API --

/// Extract all symbols (functions, structs, etc.) from source code.
///
/// # Errors
///
/// Returns `TreeSitterError` if parsing fails or the language is unsupported.
pub fn extract_symbols(
    source: &str,
    lang: SupportedLanguage,
) -> Result<Vec<Symbol>, TreeSitterError> {
    let mut parser = CodeParser::new();
    let tree = parser.parse_str(source, lang)?;
    match lang {
        SupportedLanguage::Rust => extract_rust_symbols(&tree, source),
        SupportedLanguage::Python => extract_python_symbols(&tree, source),
        SupportedLanguage::TypeScript | SupportedLanguage::Tsx => {
            extract_typescript_symbols(&tree, source, lang)
        }
    }
}

/// Extract import/dependency edges from source code.
///
/// # Errors
///
/// Returns `TreeSitterError` if parsing fails or the language is unsupported.
pub fn extract_imports(
    source: &str,
    lang: SupportedLanguage,
    source_file: &str,
) -> Result<Vec<ImportEdge>, TreeSitterError> {
    let mut parser = CodeParser::new();
    let tree = parser.parse_str(source, lang)?;
    match lang {
        SupportedLanguage::Rust => extract_rust_imports(&tree, source, source_file),
        SupportedLanguage::Python => extract_python_imports(&tree, source, source_file),
        SupportedLanguage::TypeScript | SupportedLanguage::Tsx => {
            extract_typescript_imports(&tree, source, source_file, lang)
        }
    }
}

// -- Tests --

#[cfg(test)]
mod tests {
    use super::*;

    // -- AST Parsing --

    #[test]
    fn parse_rust_source() {
        let mut p = CodeParser::new();
        let tree = p
            .parse_str("fn main() {}", SupportedLanguage::Rust)
            .unwrap();
        assert_eq!(tree.root_node().kind(), "source_file");
    }

    #[test]
    fn parse_python_source() {
        let mut p = CodeParser::new();
        let tree = p
            .parse_str("def hello(): pass", SupportedLanguage::Python)
            .unwrap();
        assert_eq!(tree.root_node().kind(), "module");
    }

    #[test]
    fn parse_typescript_source() {
        let mut p = CodeParser::new();
        let tree = p
            .parse_str("function hello(): void {}", SupportedLanguage::TypeScript)
            .unwrap();
        assert_eq!(tree.root_node().kind(), "program");
    }

    #[test]
    fn parse_tsx_source() {
        let mut p = CodeParser::new();
        let tree = p
            .parse_str("const App = () => <div/>;", SupportedLanguage::Tsx)
            .unwrap();
        assert_eq!(tree.root_node().kind(), "program");
    }

    #[test]
    fn unsupported_extension() {
        assert!(SupportedLanguage::from_extension("java").is_err());
    }

    #[test]
    fn parser_reuse() {
        let mut p = CodeParser::new();
        let t1 = p
            .parse_str("fn main() {}", SupportedLanguage::Rust)
            .unwrap();
        let t2 = p
            .parse_str("def hello(): pass", SupportedLanguage::Python)
            .unwrap();
        assert_eq!(t1.root_node().kind(), "source_file");
        assert_eq!(t2.root_node().kind(), "module");
    }

    // -- Rust symbols --

    #[test]
    fn rust_function() {
        let syms =
            extract_symbols("pub fn add(a: i32) -> i32 { a }", SupportedLanguage::Rust).unwrap();
        let fns: Vec<_> = syms
            .iter()
            .filter(|s| s.kind == SymbolKind::Function)
            .collect();
        assert_eq!(fns.len(), 1);
        assert_eq!(fns[0].name, "add");
        assert_eq!(fns[0].visibility, Visibility::Public);
    }

    #[test]
    fn rust_private_function() {
        let syms = extract_symbols("fn helper() {}", SupportedLanguage::Rust).unwrap();
        let fns: Vec<_> = syms
            .iter()
            .filter(|s| s.kind == SymbolKind::Function)
            .collect();
        assert_eq!(fns[0].visibility, Visibility::Private);
    }

    #[test]
    fn rust_struct() {
        let syms = extract_symbols(
            "pub struct Config { pub name: String }",
            SupportedLanguage::Rust,
        )
        .unwrap();
        let s: Vec<_> = syms
            .iter()
            .filter(|s| s.kind == SymbolKind::Struct)
            .collect();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].name, "Config");
        assert_eq!(s[0].visibility, Visibility::Public);
    }

    #[test]
    fn rust_enum() {
        let syms =
            extract_symbols("pub enum Color { Red, Green }", SupportedLanguage::Rust).unwrap();
        let e: Vec<_> = syms.iter().filter(|s| s.kind == SymbolKind::Enum).collect();
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].name, "Color");
    }

    #[test]
    fn rust_use() {
        let syms =
            extract_symbols("use std::collections::HashMap;", SupportedLanguage::Rust).unwrap();
        let i: Vec<_> = syms
            .iter()
            .filter(|s| s.kind == SymbolKind::Import)
            .collect();
        assert_eq!(i.len(), 1);
        assert_eq!(i[0].name, "std::collections::HashMap");
    }

    #[test]
    fn rust_mod() {
        let syms = extract_symbols("pub mod utils;", SupportedLanguage::Rust).unwrap();
        let m: Vec<_> = syms
            .iter()
            .filter(|s| s.kind == SymbolKind::Module)
            .collect();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].name, "utils");
        assert_eq!(m[0].visibility, Visibility::Public);
    }

    #[test]
    fn rust_const() {
        let syms = extract_symbols("pub const MAX: usize = 100;", SupportedLanguage::Rust).unwrap();
        let c: Vec<_> = syms
            .iter()
            .filter(|s| s.kind == SymbolKind::Constant)
            .collect();
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].name, "MAX");
    }

    #[test]
    fn rust_type_alias() {
        let syms = extract_symbols(
            "pub type Result<T> = std::result::Result<T, Error>;",
            SupportedLanguage::Rust,
        )
        .unwrap();
        let t: Vec<_> = syms
            .iter()
            .filter(|s| s.kind == SymbolKind::TypeAlias)
            .collect();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].name, "Result");
    }

    #[test]
    fn rust_method() {
        let src = "struct Foo;\nimpl Foo {\n    pub fn bar(&self) {}\n    fn baz(&self) {}\n}";
        let syms = extract_symbols(src, SupportedLanguage::Rust).unwrap();
        let m: Vec<_> = syms
            .iter()
            .filter(|s| s.kind == SymbolKind::Method)
            .collect();
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].name, "bar");
        assert_eq!(m[0].visibility, Visibility::Public);
        assert_eq!(m[1].name, "baz");
        assert_eq!(m[1].visibility, Visibility::Private);
    }

    #[test]
    fn rust_trait_impl_macro_and_doc() {
        let src = "/// public trait\npub trait Service {}\nimpl Service for App {}\nmacro_rules! trace { () => {} }";
        let syms = extract_symbols(src, SupportedLanguage::Rust).unwrap();
        assert!(syms.iter().any(|s| {
            s.kind == SymbolKind::Trait
                && s.name == "Service"
                && s.doc.as_deref() == Some("public trait")
        }));
        assert!(
            syms.iter()
                .any(|s| s.kind == SymbolKind::Impl && s.name == "App")
        );
        assert!(
            syms.iter()
                .any(|s| s.kind == SymbolKind::Macro && s.name == "trace")
        );
    }

    // -- Python symbols --

    #[test]
    fn python_function() {
        let syms =
            extract_symbols("def greet(name):\n    pass", SupportedLanguage::Python).unwrap();
        let f: Vec<_> = syms
            .iter()
            .filter(|s| s.kind == SymbolKind::Function)
            .collect();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].name, "greet");
        assert_eq!(f[0].visibility, Visibility::Public);
    }

    #[test]
    fn python_class() {
        let syms = extract_symbols("class User:\n    pass", SupportedLanguage::Python).unwrap();
        let c: Vec<_> = syms
            .iter()
            .filter(|s| s.kind == SymbolKind::Class)
            .collect();
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].name, "User");
    }

    #[test]
    fn python_import() {
        let syms = extract_symbols("from os.path import join", SupportedLanguage::Python).unwrap();
        let count = syms.iter().filter(|s| s.kind == SymbolKind::Import).count();
        assert_eq!(count, 1);
    }

    #[test]
    fn python_method_and_constant() {
        let src = "MAX_SIZE = 10\nclass User:\n    def name(self):\n        return 'u'";
        let syms = extract_symbols(src, SupportedLanguage::Python).unwrap();
        assert!(
            syms.iter()
                .any(|s| s.kind == SymbolKind::Constant && s.name == "MAX_SIZE")
        );
        assert!(syms.iter().any(|s| {
            s.kind == SymbolKind::Method && s.name == "name" && s.parent.as_deref() == Some("User")
        }));
    }

    // -- TypeScript symbols --

    #[test]
    fn typescript_function() {
        let syms = extract_symbols(
            "export function hello(): void {}",
            SupportedLanguage::TypeScript,
        )
        .unwrap();
        let f: Vec<_> = syms
            .iter()
            .filter(|s| s.kind == SymbolKind::Function)
            .collect();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].name, "hello");
        assert_eq!(f[0].visibility, Visibility::Public);
    }

    #[test]
    fn typescript_class() {
        let syms = extract_symbols(
            "export class User { name: string; }",
            SupportedLanguage::TypeScript,
        )
        .unwrap();
        let c: Vec<_> = syms
            .iter()
            .filter(|s| s.kind == SymbolKind::Class)
            .collect();
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].name, "User");
        assert_eq!(c[0].visibility, Visibility::Public);
    }

    #[test]
    fn typescript_interface() {
        let syms = extract_symbols(
            "export interface Config { name: string }",
            SupportedLanguage::TypeScript,
        )
        .unwrap();
        let i: Vec<_> = syms
            .iter()
            .filter(|s| s.kind == SymbolKind::Interface)
            .collect();
        assert_eq!(i.len(), 1);
        assert_eq!(i[0].name, "Config");
        assert_eq!(i[0].visibility, Visibility::Public);
    }

    #[test]
    fn typescript_enum() {
        let syms =
            extract_symbols("enum Direction { Up, Down }", SupportedLanguage::TypeScript).unwrap();
        let e: Vec<_> = syms.iter().filter(|s| s.kind == SymbolKind::Enum).collect();
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].name, "Direction");
        assert_eq!(e[0].visibility, Visibility::Private);
    }

    #[test]
    fn typescript_import() {
        let syms = extract_symbols(
            "import { useState } from 'react';",
            SupportedLanguage::TypeScript,
        )
        .unwrap();
        let count = syms.iter().filter(|s| s.kind == SymbolKind::Import).count();
        assert_eq!(count, 1);
    }

    #[test]
    fn typescript_type_alias() {
        let syms =
            extract_symbols("export type ID = string;", SupportedLanguage::TypeScript).unwrap();
        let t: Vec<_> = syms
            .iter()
            .filter(|s| s.kind == SymbolKind::TypeAlias)
            .collect();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].name, "ID");
    }

    #[test]
    fn typescript_method_property_and_variable() {
        let src = "export class User { name = 's'; greet() {} }\nconst answer = 42;";
        let syms = extract_symbols(src, SupportedLanguage::TypeScript).unwrap();
        assert!(
            syms.iter()
                .any(|s| s.kind == SymbolKind::Method && s.name == "greet")
        );
        assert!(
            syms.iter()
                .any(|s| s.kind == SymbolKind::Property && s.name == "name")
        );
        assert!(
            syms.iter()
                .any(|s| s.kind == SymbolKind::Variable && s.name == "answer")
        );
    }

    // -- Rust imports --

    #[test]
    fn rust_import_simple() {
        let edges =
            extract_imports("use std::fs;", SupportedLanguage::Rust, "src/main.rs").unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].imported_path, "std::fs");
        assert_eq!(edges[0].alias, None);
    }

    #[test]
    fn rust_import_grouped() {
        let edges =
            extract_imports("use std::{io, fs};", SupportedLanguage::Rust, "src/main.rs").unwrap();
        assert_eq!(edges.len(), 2);
        assert_eq!(edges[0].imported_path, "std::io");
        assert_eq!(edges[1].imported_path, "std::fs");
    }

    #[test]
    fn rust_import_alias() {
        let edges = extract_imports(
            "use std::collections::HashMap as Map;",
            SupportedLanguage::Rust,
            "src/main.rs",
        )
        .unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].imported_path, "std::collections::HashMap");
        assert_eq!(edges[0].alias, Some("Map".to_string()));
    }

    // -- Python imports --

    #[test]
    fn python_import_simple() {
        let edges = extract_imports("import os", SupportedLanguage::Python, "main.py").unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].imported_path, "os");
    }

    #[test]
    fn python_import_from() {
        let edges = extract_imports(
            "from os.path import join",
            SupportedLanguage::Python,
            "main.py",
        )
        .unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].imported_path, "os.path.join");
    }

    #[test]
    fn python_import_alias() {
        let edges =
            extract_imports("import numpy as np", SupportedLanguage::Python, "main.py").unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].imported_path, "numpy");
        assert_eq!(edges[0].alias, Some("np".to_string()));
    }

    // -- TypeScript imports --

    #[test]
    fn typescript_import_named() {
        let edges = extract_imports(
            "import { useState } from 'react';",
            SupportedLanguage::TypeScript,
            "app.ts",
        )
        .unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].imported_path, "react");
    }

    #[test]
    fn typescript_import_default() {
        let edges = extract_imports(
            "import React from 'react';",
            SupportedLanguage::TypeScript,
            "app.ts",
        )
        .unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].imported_path, "react");
    }

    #[test]
    fn typescript_import_namespace() {
        let edges = extract_imports(
            "import * as fs from 'fs';",
            SupportedLanguage::TypeScript,
            "app.ts",
        )
        .unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].imported_path, "fs");
        assert_eq!(edges[0].alias, Some("fs".to_string()));
    }
}
