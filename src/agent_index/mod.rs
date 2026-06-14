//! Generate agent-facing index artifacts for MCP resources and tools.

use anyhow::{Context, Result};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

pub fn generate_agent_index(output: &Path) -> Result<()> {
    fs::create_dir_all(output).context("create agent_index output dir")?;

    let registry = crate::error_codes::get_registry();
    let errors: serde_json::Map<String, serde_json::Value> = registry
        .all_codes()
        .into_iter()
        .map(|code| {
            (
                code.code.clone(),
                json!({
                    "code": code.code,
                    "title": code.title,
                    "explanation": code.explanation,
                    "causes": code.causes,
                    "solutions": code.solutions,
                    "example": code.example,
                    "rust_codes": code.rust_codes,
                }),
            )
        })
        .collect();
    write_json(output.join("errors.json"), &json!(errors))?;

    let stdlib = generate_stdlib_index()?;
    write_json(output.join("stdlib.json"), &stdlib)?;

    let spec = generate_spec_index()?;
    write_json(output.join("spec.json"), &spec)?;

    let changelog = json!({
        "source": "CHANGELOG.md",
        "note": "Run wj agent-index after release to refresh"
    });
    write_json(output.join("changelog.json"), &changelog)?;

    let lint_policy = json!({
        "rules": [
            {"id": "no-rust-leakage", "forbidden": [".as_str()", ".unwrap()", "explicit &"]},
            {"id": "wj-game-build", "required_command": "wj game build"}
        ]
    });
    write_json(output.join("lint_policy.json"), &lint_policy)?;

    let meta = json!({
        "generated_by": "wj agent-index",
        "windjammer_version": env!("CARGO_PKG_VERSION"),
        "generated_at": chrono::Utc::now().to_rfc3339(),
        "artifact_count": 5
    });
    write_json(output.join("index_meta.json"), &meta)?;

    println!("Agent index written to {}", output.display());
    Ok(())
}

fn write_json(path: PathBuf, value: &serde_json::Value) -> Result<()> {
    let text = serde_json::to_string_pretty(value)?;
    fs::write(&path, text).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn generate_stdlib_index() -> Result<serde_json::Value> {
    let std_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("std");
    let mut modules = Vec::new();
    if std_root.exists() {
        for entry in fs::read_dir(&std_root)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("wj") {
                modules.push(json!({
                    "module": path.file_stem().and_then(|s| s.to_str()).unwrap_or(""),
                    "path": path.display().to_string()
                }));
            }
        }
    }
    Ok(json!({ "modules": modules, "readme": "std/README.md" }))
}

fn generate_spec_index() -> Result<serde_json::Value> {
    let spec_md = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/LANGUAGE_SPEC_TESTS.md");
    Ok(json!({
        "index_file": spec_md.display().to_string(),
        "exists": spec_md.exists(),
        "categories": ["parser", "analyzer", "codegen", "inference", "pattern_matching"]
    }))
}
