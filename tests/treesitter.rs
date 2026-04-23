#[path = "support/result.rs"]
mod result_support;

use cortex_plugin_dev::treesitter::{
    CodeParser, SupportedLanguage, SymbolKind, Visibility, extract_imports, extract_symbols,
};
use result_support::ResultTestExt;

#[test]
fn parse_rust_source() {
    let mut p = CodeParser::new();
    let tree = p
        .parse_str("fn main() {}", SupportedLanguage::Rust)
        .or_panic();
    assert_eq!(tree.root_node().kind(), "source_file");
}

#[test]
fn parse_python_source() {
    let mut p = CodeParser::new();
    let tree = p
        .parse_str("def hello(): pass", SupportedLanguage::Python)
        .or_panic();
    assert_eq!(tree.root_node().kind(), "module");
}

#[test]
fn parse_typescript_source() {
    let mut p = CodeParser::new();
    let tree = p
        .parse_str("function hello(): void {}", SupportedLanguage::TypeScript)
        .or_panic();
    assert_eq!(tree.root_node().kind(), "program");
}

#[test]
fn parse_tsx_source() {
    let mut p = CodeParser::new();
    let tree = p
        .parse_str("const App = () => <div/>;", SupportedLanguage::Tsx)
        .or_panic();
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
        .or_panic();
    let t2 = p
        .parse_str("def hello(): pass", SupportedLanguage::Python)
        .or_panic();
    assert_eq!(t1.root_node().kind(), "source_file");
    assert_eq!(t2.root_node().kind(), "module");
}

#[test]
fn rust_function() {
    let syms =
        extract_symbols("pub fn add(a: i32) -> i32 { a }", SupportedLanguage::Rust).or_panic();
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
    let syms = extract_symbols("fn helper() {}", SupportedLanguage::Rust).or_panic();
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
    .or_panic();
    let items: Vec<_> = syms
        .iter()
        .filter(|s| s.kind == SymbolKind::Struct)
        .collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "Config");
    assert_eq!(items[0].visibility, Visibility::Public);
}

#[test]
fn rust_enum() {
    let syms = extract_symbols("pub enum Color { Red, Green }", SupportedLanguage::Rust).or_panic();
    let items: Vec<_> = syms.iter().filter(|s| s.kind == SymbolKind::Enum).collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "Color");
}

#[test]
fn rust_use() {
    let syms =
        extract_symbols("use std::collections::HashMap;", SupportedLanguage::Rust).or_panic();
    let items: Vec<_> = syms
        .iter()
        .filter(|s| s.kind == SymbolKind::Import)
        .collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "std::collections::HashMap");
}

#[test]
fn rust_mod() {
    let syms = extract_symbols("pub mod utils;", SupportedLanguage::Rust).or_panic();
    let items: Vec<_> = syms
        .iter()
        .filter(|s| s.kind == SymbolKind::Module)
        .collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "utils");
    assert_eq!(items[0].visibility, Visibility::Public);
}

#[test]
fn rust_const() {
    let syms = extract_symbols("pub const MAX: usize = 100;", SupportedLanguage::Rust).or_panic();
    let items: Vec<_> = syms
        .iter()
        .filter(|s| s.kind == SymbolKind::Constant)
        .collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "MAX");
}

#[test]
fn rust_type_alias() {
    let syms = extract_symbols(
        "pub type Result<T> = std::result::Result<T, Error>;",
        SupportedLanguage::Rust,
    )
    .or_panic();
    let items: Vec<_> = syms
        .iter()
        .filter(|s| s.kind == SymbolKind::TypeAlias)
        .collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "Result");
}

#[test]
fn rust_method() {
    let src = "struct Foo;\nimpl Foo {\n    pub fn bar(&self) {}\n    fn baz(&self) {}\n}";
    let syms = extract_symbols(src, SupportedLanguage::Rust).or_panic();
    let items: Vec<_> = syms
        .iter()
        .filter(|s| s.kind == SymbolKind::Method)
        .collect();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].name, "bar");
    assert_eq!(items[0].visibility, Visibility::Public);
    assert_eq!(items[1].name, "baz");
    assert_eq!(items[1].visibility, Visibility::Private);
}

#[test]
fn rust_trait_impl_macro_and_doc() {
    let src = "/// public trait\npub trait Service {}\nimpl Service for App {}\nmacro_rules! trace { () => {} }";
    let syms = extract_symbols(src, SupportedLanguage::Rust).or_panic();
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

#[test]
fn python_function() {
    let syms = extract_symbols("def greet(name):\n    pass", SupportedLanguage::Python).or_panic();
    let items: Vec<_> = syms
        .iter()
        .filter(|s| s.kind == SymbolKind::Function)
        .collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "greet");
    assert_eq!(items[0].visibility, Visibility::Public);
}

#[test]
fn python_class() {
    let syms = extract_symbols("class User:\n    pass", SupportedLanguage::Python).or_panic();
    let items: Vec<_> = syms
        .iter()
        .filter(|s| s.kind == SymbolKind::Class)
        .collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "User");
}

#[test]
fn python_import() {
    let syms = extract_symbols("from os.path import join", SupportedLanguage::Python).or_panic();
    let count = syms.iter().filter(|s| s.kind == SymbolKind::Import).count();
    assert_eq!(count, 1);
}

#[test]
fn python_method_and_constant() {
    let src = "MAX_SIZE = 10\nclass User:\n    def name(self):\n        return 'u'";
    let syms = extract_symbols(src, SupportedLanguage::Python).or_panic();
    assert!(
        syms.iter()
            .any(|s| s.kind == SymbolKind::Constant && s.name == "MAX_SIZE")
    );
    assert!(syms.iter().any(|s| s.kind == SymbolKind::Method
        && s.name == "name"
        && s.parent.as_deref() == Some("User")));
}

#[test]
fn typescript_function() {
    let syms = extract_symbols(
        "export function hello(): void {}",
        SupportedLanguage::TypeScript,
    )
    .or_panic();
    let items: Vec<_> = syms
        .iter()
        .filter(|s| s.kind == SymbolKind::Function)
        .collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "hello");
    assert_eq!(items[0].visibility, Visibility::Public);
}

#[test]
fn typescript_class() {
    let syms = extract_symbols(
        "export class User { name: string; }",
        SupportedLanguage::TypeScript,
    )
    .or_panic();
    let items: Vec<_> = syms
        .iter()
        .filter(|s| s.kind == SymbolKind::Class)
        .collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "User");
    assert_eq!(items[0].visibility, Visibility::Public);
}

#[test]
fn typescript_interface() {
    let syms = extract_symbols(
        "export interface Config { name: string }",
        SupportedLanguage::TypeScript,
    )
    .or_panic();
    let items: Vec<_> = syms
        .iter()
        .filter(|s| s.kind == SymbolKind::Interface)
        .collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "Config");
    assert_eq!(items[0].visibility, Visibility::Public);
}

#[test]
fn typescript_enum() {
    let syms =
        extract_symbols("enum Direction { Up, Down }", SupportedLanguage::TypeScript).or_panic();
    let items: Vec<_> = syms.iter().filter(|s| s.kind == SymbolKind::Enum).collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "Direction");
    assert_eq!(items[0].visibility, Visibility::Private);
}

#[test]
fn typescript_import() {
    let syms = extract_symbols(
        "import { useState } from 'react';",
        SupportedLanguage::TypeScript,
    )
    .or_panic();
    let count = syms.iter().filter(|s| s.kind == SymbolKind::Import).count();
    assert_eq!(count, 1);
}

#[test]
fn typescript_type_alias() {
    let syms =
        extract_symbols("export type ID = string;", SupportedLanguage::TypeScript).or_panic();
    let items: Vec<_> = syms
        .iter()
        .filter(|s| s.kind == SymbolKind::TypeAlias)
        .collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "ID");
}

#[test]
fn typescript_method_property_and_variable() {
    let src = "export class User { name = 's'; greet() {} }\nconst answer = 42;";
    let syms = extract_symbols(src, SupportedLanguage::TypeScript).or_panic();
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

#[test]
fn rust_import_simple() {
    let edges = extract_imports("use std::fs;", SupportedLanguage::Rust, "src/main.rs").or_panic();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].imported_path, "std::fs");
    assert_eq!(edges[0].alias, None);
}

#[test]
fn rust_import_grouped() {
    let edges =
        extract_imports("use std::{io, fs};", SupportedLanguage::Rust, "src/main.rs").or_panic();
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
    .or_panic();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].imported_path, "std::collections::HashMap");
    assert_eq!(edges[0].alias, Some("Map".to_string()));
}

#[test]
fn python_import_simple() {
    let edges = extract_imports("import os", SupportedLanguage::Python, "main.py").or_panic();
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
    .or_panic();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].imported_path, "os.path.join");
}

#[test]
fn python_import_alias() {
    let edges =
        extract_imports("import numpy as np", SupportedLanguage::Python, "main.py").or_panic();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].imported_path, "numpy");
    assert_eq!(edges[0].alias, Some("np".to_string()));
}

#[test]
fn typescript_import_named() {
    let edges = extract_imports(
        "import { useState } from 'react';",
        SupportedLanguage::TypeScript,
        "app.ts",
    )
    .or_panic();
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
    .or_panic();
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
    .or_panic();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].imported_path, "fs");
    assert_eq!(edges[0].alias, Some("fs".to_string()));
}
