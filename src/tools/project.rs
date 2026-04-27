use cortex_sdk::{Tool, ToolCapabilities, ToolError, ToolResult};
use ignore::WalkBuilder;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const MAX_FILES: usize = 20_000;

#[derive(Default)]
struct ProjectFacts {
    root: PathBuf,
    markers: Vec<&'static str>,
    languages: BTreeMap<String, usize>,
    entry_points: Vec<String>,
    test_commands: Vec<String>,
    package_managers: Vec<&'static str>,
}

fn path_input(input: &serde_json::Value) -> PathBuf {
    input
        .get("path")
        .and_then(serde_json::Value::as_str)
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
}

fn bool_input(input: &serde_json::Value, key: &str) -> bool {
    input
        .get(key)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn format_input<'a>(input: &'a serde_json::Value, default: &'a str) -> &'a str {
    input
        .get("format")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(default)
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

fn collect_facts(root: &Path) -> Result<ProjectFacts, ToolError> {
    ensure_dir(root)?;
    let mut facts = ProjectFacts {
        root: root.to_path_buf(),
        ..ProjectFacts::default()
    };

    for (file, marker) in [
        ("Cargo.toml", "rust/cargo"),
        ("package.json", "node/package"),
        ("pyproject.toml", "python/pyproject"),
        ("setup.py", "python/setup"),
        ("go.mod", "go/module"),
        ("tsconfig.json", "typescript"),
        ("Dockerfile", "docker"),
        ("docker-compose.yml", "docker-compose"),
    ] {
        if root.join(file).exists() {
            facts.markers.push(marker);
        }
    }

    for (file, manager) in [
        ("Cargo.lock", "cargo"),
        ("package-lock.json", "npm"),
        ("pnpm-lock.yaml", "pnpm"),
        ("yarn.lock", "yarn"),
        ("uv.lock", "uv"),
        ("poetry.lock", "poetry"),
        ("go.sum", "go"),
    ] {
        if root.join(file).exists() {
            facts.package_managers.push(manager);
        }
    }

    let mut seen = 0usize;
    for entry in WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .build()
        .flatten()
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        seen += 1;
        if seen > MAX_FILES {
            break;
        }
        if let Some(language) = language_for_path(path) {
            *facts.languages.entry(language.to_string()).or_default() += 1;
        }
        if is_entry_point(path)
            && let Ok(relative) = path.strip_prefix(root)
        {
            facts.entry_points.push(relative.display().to_string());
        }
    }
    facts.entry_points.sort();
    facts.entry_points.truncate(30);
    facts.test_commands = discover_test_commands(root);
    Ok(facts)
}

fn language_for_path(path: &Path) -> Option<&'static str> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("rs") => Some("Rust"),
        Some("py") => Some("Python"),
        Some("ts" | "tsx") => Some("TypeScript"),
        Some("js" | "jsx" | "mjs" | "cjs") => Some("JavaScript"),
        Some("go") => Some("Go"),
        Some("java") => Some("Java"),
        Some("kt" | "kts") => Some("Kotlin"),
        Some("swift") => Some("Swift"),
        Some("c" | "h") => Some("C"),
        Some("cc" | "cpp" | "hpp" | "cxx") => Some("C++"),
        Some("toml") => Some("TOML"),
        Some("yaml" | "yml") => Some("YAML"),
        Some("json") => Some("JSON"),
        Some("md") => Some("Markdown"),
        _ => None,
    }
}

fn is_entry_point(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    matches!(
        name,
        "main.rs"
            | "lib.rs"
            | "main.py"
            | "app.py"
            | "index.ts"
            | "index.tsx"
            | "index.js"
            | "main.go"
            | "Dockerfile"
    )
}

fn discover_test_commands(root: &Path) -> Vec<String> {
    let mut commands = Vec::new();
    if root.join("Cargo.toml").exists() {
        commands.push("cargo test".to_string());
        commands.push("cargo clippy --all-targets --all-features -- -D warnings".to_string());
    }
    if root.join("package.json").exists() {
        let package = std::fs::read_to_string(root.join("package.json")).unwrap_or_default();
        if package.contains("\"test\"") {
            commands.push("npm test".to_string());
        }
        if root.join("pnpm-lock.yaml").exists() {
            commands.push("pnpm test".to_string());
        }
        if root.join("yarn.lock").exists() {
            commands.push("yarn test".to_string());
        }
    }
    if root.join("pyproject.toml").exists() || root.join("pytest.ini").exists() {
        commands.push("pytest".to_string());
    }
    if root.join("go.mod").exists() {
        commands.push("go test ./...".to_string());
    }
    commands.sort();
    commands.dedup();
    commands
}

fn render_facts_text(facts: &ProjectFacts) -> String {
    let mut lines = vec![format!("Project: {}", facts.root.display())];
    if !facts.markers.is_empty() {
        lines.push(format!("Markers: {}", facts.markers.join(", ")));
    }
    if !facts.package_managers.is_empty() {
        lines.push(format!(
            "Package managers: {}",
            facts.package_managers.join(", ")
        ));
    }
    if !facts.languages.is_empty() {
        lines.push("Languages:".to_string());
        for (language, count) in &facts.languages {
            lines.push(format!("- {language}: {count} files"));
        }
    }
    if !facts.entry_points.is_empty() {
        lines.push("Entry points:".to_string());
        for entry in &facts.entry_points {
            lines.push(format!("- {entry}"));
        }
    }
    if !facts.test_commands.is_empty() {
        lines.push("Likely test commands:".to_string());
        for command in &facts.test_commands {
            lines.push(format!("- {command}"));
        }
    }
    lines.join("\n")
}

fn facts_json(facts: &ProjectFacts) -> serde_json::Value {
    serde_json::json!({
        "root": facts.root,
        "markers": facts.markers,
        "languages": facts.languages,
        "entry_points": facts.entry_points,
        "test_commands": facts.test_commands,
        "package_managers": facts.package_managers,
    })
}

pub struct ProjectMapTool;

impl Tool for ProjectMapTool {
    fn name(&self) -> &'static str {
        "project_map"
    }

    fn description(&self) -> &'static str {
        "Summarize project shape: languages, entry points, package managers, and likely test commands.\n\n\
         Use at the start of coding tasks to orient before editing."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Project root (default: current directory)" },
                "format": { "type": "string", "enum": ["text", "json"], "description": "Output format (default: text)" }
            }
        })
    }

    fn capabilities(&self) -> ToolCapabilities {
        super::caps([super::read_file_effect("project root")])
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let root = path_input(&input);
        let facts = collect_facts(&root)?;
        if format_input(&input, "text") == "json" {
            serde_json::to_string_pretty(&facts_json(&facts))
                .map(ToolResult::success)
                .map_err(|e| ToolError::ExecutionFailed(format!("json encode failed: {e}")))
        } else {
            Ok(ToolResult::success(render_facts_text(&facts)))
        }
    }
}

pub struct TestDiscoverTool;

impl Tool for TestDiscoverTool {
    fn name(&self) -> &'static str {
        "test_discover"
    }

    fn description(&self) -> &'static str {
        "Discover likely test and lint commands for a project without running them.\n\n\
         Use before diagnostics or release validation to choose the right commands."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Project root (default: current directory)" },
                "include_lints": { "type": "boolean", "description": "Include lint/type-check commands when known (default: false)" }
            }
        })
    }

    fn capabilities(&self) -> ToolCapabilities {
        super::caps([super::read_file_effect("project root")])
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let root = path_input(&input);
        ensure_dir(&root)?;
        let include_lints = bool_input(&input, "include_lints");
        let mut commands = discover_test_commands(&root);
        if include_lints {
            if root.join("tsconfig.json").exists() {
                commands.push("npx tsc --noEmit".to_string());
            }
            if root.join("pyproject.toml").exists() {
                commands.push("pyright".to_string());
            }
        } else {
            commands.retain(|command| {
                !command.contains("clippy") && !command.contains("tsc") && command != "pyright"
            });
        }
        commands.sort();
        commands.dedup();
        if commands.is_empty() {
            Ok(ToolResult::success("No test commands detected".to_string()))
        } else {
            Ok(ToolResult::success(commands.join("\n")))
        }
    }
}

pub struct DependencyAuditTool;

impl Tool for DependencyAuditTool {
    fn name(&self) -> &'static str {
        "dependency_audit"
    }

    fn description(&self) -> &'static str {
        "Inspect dependency manifests and lockfiles for ecosystem, manager, and review targets.\n\n\
         This is a local structural audit, not a vulnerability database lookup."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Project root (default: current directory)" },
                "format": { "type": "string", "enum": ["text", "json"], "description": "Output format (default: text)" }
            }
        })
    }

    fn capabilities(&self) -> ToolCapabilities {
        super::caps([super::read_file_effect("dependency manifests")])
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let root = path_input(&input);
        ensure_dir(&root)?;
        let manifests = dependency_files(&root);
        if format_input(&input, "text") == "json" {
            let json = serde_json::json!({ "root": root, "files": manifests });
            serde_json::to_string_pretty(&json)
                .map(ToolResult::success)
                .map_err(|e| ToolError::ExecutionFailed(format!("json encode failed: {e}")))
        } else if manifests.is_empty() {
            Ok(ToolResult::success(
                "No dependency manifests detected".to_string(),
            ))
        } else {
            Ok(ToolResult::success(format!(
                "Dependency files:\n{}",
                manifests
                    .iter()
                    .map(|file| format!("- {file}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            )))
        }
    }
}

fn dependency_files(root: &Path) -> Vec<String> {
    [
        "Cargo.toml",
        "Cargo.lock",
        "package.json",
        "package-lock.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "pyproject.toml",
        "poetry.lock",
        "uv.lock",
        "requirements.txt",
        "go.mod",
        "go.sum",
        "pom.xml",
        "build.gradle",
        "build.gradle.kts",
    ]
    .into_iter()
    .filter(|file| root.join(file).exists())
    .map(str::to_string)
    .collect()
}
