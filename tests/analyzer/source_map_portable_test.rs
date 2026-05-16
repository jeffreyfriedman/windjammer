#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

// Verifies that the SourceMap type stores relative paths when a workspace root is set.

use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_source_maps_use_relative_paths() {
    let temp_dir = tempdir().expect("temp dir");
    let workspace = temp_dir.path();
    fs::create_dir_all(workspace.join("src")).expect("src");
    fs::create_dir_all(workspace.join("build")).expect("build");
    fs::write(workspace.join("src/test.wj"), "fn main() {}").expect("write wj");

    let mut sm = windjammer::source_map::SourceMap::new();
    sm.set_workspace_root(workspace);
    sm.add_mapping(
        workspace.join("build/test.rs"),
        1,
        1,
        workspace.join("src/test.wj"),
        2,
        1,
    );

    let map_path = workspace.join("test.rs.map");
    sm.save_to_file(&map_path).expect("save");

    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&map_path).expect("read")).expect("parse");

    let mappings = json["mappings"].as_array().expect("mappings");
    let m0 = mappings[0].as_object().expect("mapping object");
    let rust_file = m0["rust_file"].as_str().expect("rust_file str");
    let wj_file = m0["wj_file"].as_str().expect("wj_file str");

    assert!(
        !rust_file.starts_with('/')
            && !rust_file.starts_with("C:\\")
            && !rust_file.starts_with("c:\\"),
        "rust_file should be relative: {}",
        rust_file
    );
    assert!(
        !wj_file.starts_with('/') && !wj_file.starts_with("C:\\") && !wj_file.starts_with("c:\\"),
        "wj_file should be relative: {}",
        wj_file
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_source_map_resolves_across_machines() {
    let temp_dir = tempdir().expect("temp dir");
    let workspace = temp_dir.path();
    fs::create_dir_all(workspace.join("src")).expect("src");
    fs::create_dir_all(workspace.join("build")).expect("build");
    fs::write(workspace.join("src/test.wj"), "fn main() {}").expect("write wj");
    fs::write(workspace.join("build/test.rs"), "fn main() {}").expect("write rs");

    let mut sm = windjammer::source_map::SourceMap::new();
    sm.set_workspace_root(workspace);
    sm.add_mapping(
        workspace.join("build/test.rs"),
        1,
        1,
        workspace.join("src/test.wj"),
        2,
        1,
    );
    let map_path = workspace.join("test.rs.map");
    sm.save_to_file(&map_path).expect("save");

    let source_map =
        windjammer::source_map::SourceMap::load_from_file(&map_path).expect("load source map");

    let rust_file = Path::new("build/test.rs");
    let mapping = source_map.lookup(rust_file, 1);
    assert!(mapping.is_some(), "lookup line 1");

    let mapping = mapping.unwrap();
    assert!(
        mapping.wj_file.to_string_lossy().contains("test.wj"),
        "wj path"
    );

    let resolved_wj = workspace.join(&mapping.wj_file);
    assert!(resolved_wj.exists(), "resolved wj: {:?}", resolved_wj);
}
