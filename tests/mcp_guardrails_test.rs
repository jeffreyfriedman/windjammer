//! CI guardrails for MCP tool catalog and agent index freshness.

use std::fs;
use std::path::PathBuf;

#[test]
fn test_mcp_manifest_matches_catalog_count() {
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("crates/windjammer-mcp/tools/manifest.json");
    let text = fs::read_to_string(&manifest_path).expect("manifest.json must exist");
    let json: serde_json::Value = serde_json::from_str(&text).expect("valid manifest json");
    let count = json["count"].as_u64().expect("count field");
    let tools = json["tools"].as_array().expect("tools array");
    assert_eq!(count as usize, tools.len());
    assert_eq!(tools.len(), 13, "expected 13 MCP tools");
}

#[test]
fn test_agent_index_errors_cover_registry() {
    let index_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("agent_index/errors.json");
    if !index_path.exists() {
        panic!("Run: cargo build --release && ./target/release/wj agent-index -o agent_index");
    }
    let text = fs::read_to_string(index_path).expect("read errors.json");
    let json: serde_json::Value = serde_json::from_str(&text).expect("parse errors.json");
    let registry = windjammer::error_codes::get_registry();
    for code in registry.all_codes() {
        assert!(
            json.get(&code.code).is_some(),
            "errors.json missing {}",
            code.code
        );
    }
}

#[test]
fn test_error_spec_yaml_lists_all_registry_codes() {
    let spec_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/errors/spec.yaml");
    let text = fs::read_to_string(&spec_path).expect("read spec.yaml");
    let registry = windjammer::error_codes::get_registry();
    for code in registry.all_codes() {
        assert!(
            text.contains(&code.code),
            "docs/errors/spec.yaml missing {}",
            code.code
        );
    }
}

#[test]
fn test_agent_index_meta_exists_when_errors_exist() {
    let meta = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("agent_index/index_meta.json");
    let errors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("agent_index/errors.json");
    if errors.exists() {
        assert!(meta.exists(), "index_meta.json missing alongside errors.json");
    }
}

#[test]
fn test_ide_analysis_matches_mcp_analyze_types_fixture() {
    use std::path::PathBuf;
    use windjammer::ide_analysis::{analyze_source, IdeAnalysisOptions};

    let source = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
    let direct = analyze_source(
        source,
        IdeAnalysisOptions {
            enable_lint: false,
            file_path: PathBuf::from("fixture.wj"),
        },
    );
    assert!(direct.success);
    assert_eq!(
        direct.inferred_types.get("add::return"),
        Some(&"i32".to_string())
    );
}
