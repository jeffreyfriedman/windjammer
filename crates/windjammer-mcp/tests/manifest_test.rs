//! Manifest drift guardrail: committed manifest matches tool catalog.

use std::fs;
use std::path::PathBuf;

use windjammer_mcp::tools::catalog::all_tool_specs;

#[test]
fn manifest_tool_names_match_catalog() {
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tools/manifest.json");
    let text = fs::read_to_string(&manifest_path).expect("manifest.json");
    let json: serde_json::Value = serde_json::from_str(&text).expect("parse manifest");
    let manifest_names: Vec<String> = json["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();

    let catalog_names: Vec<String> = all_tool_specs()
        .iter()
        .map(|s| s.name.to_string())
        .collect();

    assert_eq!(
        manifest_names, catalog_names,
        "manifest.json out of sync with catalog.rs — regenerate manifest"
    );
}
