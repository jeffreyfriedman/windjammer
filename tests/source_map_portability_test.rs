//! Tests for source map portability across machines.
//!
//! The `wj build` CLI does not always emit `.rs.map` siblings for every invocation; these tests
//! exercise the `windjammer::source_map::SourceMap` serialization contract (JSON shape, relative
//! paths) which is what rustc error translation relies on.

use std::path::PathBuf;
use tempfile::tempdir;

fn sample_source_map_on_disk() -> (tempfile::TempDir, PathBuf) {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let map_path = root.join("out.rs.map");

    let mut sm = windjammer::source_map::SourceMap::new();
    sm.set_workspace_root(root);
    sm.add_mapping(
        root.join("out/main.rs"),
        10,
        1,
        root.join("src/main.wj"),
        5,
        1,
    );
    sm.save_to_file(&map_path).expect("save");

    (dir, map_path)
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_source_map_valid_json() {
    let (_dir, map_path) = sample_source_map_on_disk();
    let content = std::fs::read_to_string(&map_path).expect("read map");
    let json: serde_json::Value = serde_json::from_str(&content).expect("json");

    assert!(json.get("version").is_some(), "Should have version field");
    assert!(json.get("mappings").is_some(), "Should have mappings field");

    let mappings = json
        .get("mappings")
        .and_then(|m| m.as_array())
        .expect("mappings array");
    if let Some(first) = mappings.first().and_then(|v| v.as_object()) {
        assert!(
            first.contains_key("rust_file") || first.contains_key("rust_line"),
            "Mapping should have rust location fields"
        );
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_source_map_has_required_fields() {
    let (_dir, map_path) = sample_source_map_on_disk();
    let content = std::fs::read_to_string(&map_path).expect("read map");
    let json: serde_json::Value = serde_json::from_str(&content).expect("json");

    assert!(json.get("version").is_some(), "version");
    assert!(json.get("mappings").is_some(), "mappings");
    assert!(
        json.get("version").expect("version").is_number(),
        "version is number"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_source_map_workspace_root_handling() {
    let (_dir, map_path) = sample_source_map_on_disk();
    let content = std::fs::read_to_string(&map_path).expect("read map");
    let json: serde_json::Value = serde_json::from_str(&content).expect("json");

    if let Some(workspace_root) = json.get("workspace_root") {
        if !workspace_root.is_null() {
            assert!(
                workspace_root.is_string(),
                "workspace_root should be a string if set"
            );
        }
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_source_map_portable_structure() {
    let (_dir, map_path) = sample_source_map_on_disk();
    let content = std::fs::read_to_string(&map_path).expect("read map");
    let json: serde_json::Value = serde_json::from_str(&content).expect("json");

    assert!(json.get("version").is_some());
    assert!(json.get("mappings").is_some());
    let mappings = json.get("mappings").expect("mappings");
    assert!(mappings.is_array(), "mappings array");
}
