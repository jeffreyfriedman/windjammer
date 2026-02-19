/// TDD Test: mod.rs should auto-generate pub use re-exports
///
/// Bug: When a module has a mod.wj with explicit pub mod declarations but no
/// pub use declarations, the generated mod.rs only has `pub mod` without
/// re-exporting public types. This causes "unresolved import" errors when
/// other files use `use crate::module::Type;` instead of the full path
/// `use crate::module::submodule::Type;`.
///
/// Expected behavior: The compiler should auto-generate `pub use submodule::*;`
/// re-exports in mod.rs when mod.wj has no explicit pub use declarations,
/// just like it does when there's no mod.wj at all.
use std::fs;
use std::process::Command;

fn compile_project(files: &[(&str, &str)]) -> std::collections::HashMap<String, String> {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src_wj");
    let out_dir = temp_dir.path().join("out");
    fs::create_dir_all(&out_dir).unwrap();

    for (path, content) in files {
        let full_path = src_dir.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&full_path, content).unwrap();
    }

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            src_dir.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        eprintln!("Compiler stderr:\n{}", stderr);
        eprintln!("Compiler stdout:\n{}", stdout);
    }

    // Read all generated .rs files
    let mut result = std::collections::HashMap::new();
    fn read_dir_recursive(
        dir: &std::path::Path,
        base: &std::path::Path,
        result: &mut std::collections::HashMap<String, String>,
    ) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    read_dir_recursive(&path, base, result);
                } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                    let rel = path
                        .strip_prefix(base)
                        .unwrap()
                        .to_string_lossy()
                        .to_string();
                    let content = fs::read_to_string(&path).unwrap();
                    result.insert(rel, content);
                }
            }
        }
    }
    read_dir_recursive(&out_dir, &out_dir, &mut result);
    result
}

#[test]
fn test_mod_rs_reexports_when_mod_wj_has_no_pub_use() {
    // Module has mod.wj with pub mod but NO pub use
    let files = &[
        ("mod.wj", "pub mod mymod\n"),
        (
            "mymod/mod.wj",
            "pub mod tile_id\npub mod edge_type\npub mod tile_rule\n",
        ),
        (
            "mymod/tile_id.wj",
            "pub struct TileId {\n    pub value: i32,\n}\n",
        ),
        (
            "mymod/edge_type.wj",
            "pub struct EdgeType {\n    pub value: i32,\n}\n",
        ),
        (
            "mymod/tile_rule.wj",
            "pub struct TileRule {\n    pub name: String,\n}\n",
        ),
    ];

    let output = compile_project(files);

    let mod_rs = output
        .get("mymod/mod.rs")
        .expect("Should generate mymod/mod.rs");

    println!("Generated mymod/mod.rs:\n{}", mod_rs);

    // Should have pub mod declarations
    assert!(
        mod_rs.contains("pub mod tile_id;"),
        "Should have pub mod tile_id"
    );
    assert!(
        mod_rs.contains("pub mod edge_type;"),
        "Should have pub mod edge_type"
    );
    assert!(
        mod_rs.contains("pub mod tile_rule;"),
        "Should have pub mod tile_rule"
    );

    // CRITICAL: Should also have re-exports so `use crate::mymod::TileId;` works
    let has_reexports = mod_rs.contains("pub use tile_id::")
        || mod_rs.contains("pub use edge_type::")
        || mod_rs.contains("pub use tile_rule::");

    assert!(
        has_reexports,
        "mod.rs should auto-generate pub use re-exports when mod.wj has no explicit pub use.\n\
         Generated mod.rs:\n{}",
        mod_rs
    );
}

#[test]
fn test_mod_rs_respects_explicit_pub_use() {
    // Module has mod.wj WITH explicit pub use
    let files = &[
        ("mod.wj", "pub mod mymod\n"),
        (
            "mymod/mod.wj",
            "pub mod tile_id\npub mod edge_type\npub use tile_id::TileId\n",
        ),
        (
            "mymod/tile_id.wj",
            "pub struct TileId {\n    pub value: i32,\n}\n",
        ),
        (
            "mymod/edge_type.wj",
            "pub struct EdgeType {\n    pub value: i32,\n}\n",
        ),
    ];

    let output = compile_project(files);

    let mod_rs = output
        .get("mymod/mod.rs")
        .expect("Should generate mymod/mod.rs");

    println!("Generated mymod/mod.rs:\n{}", mod_rs);

    // Should have the explicit pub use from mod.wj
    assert!(
        mod_rs.contains("pub use tile_id::TileId;"),
        "Should have explicit pub use from mod.wj.\nGenerated:\n{}",
        mod_rs
    );
}
