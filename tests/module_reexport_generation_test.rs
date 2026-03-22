//! TDD Test: Module re-export generation
//!
//! Bug: mod.wj files with `pub use` declarations do not generate corresponding
//! `pub use` statements in the output mod.rs. This causes E0432 errors when
//! types are imported via the parent module path (e.g., `use crate::math::Vec2`).
//!
//! Expected: `pub use submodule::MyType` in mod.wj → `pub use self::submodule::MyType;` in mod.rs

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

    let output = Command::new(env!("CARGO_BIN_EXE_wj")).args([
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
                        .replace('\\', "/");
                    let content = fs::read_to_string(&path).unwrap();
                    result.insert(rel, content);
                }
            }
        }
    }
    read_dir_recursive(&out_dir, &out_dir, &mut result);
    result
}

/// Simple re-export: pub use vec2::Vec2 in mod.wj → pub use in mod.rs
#[test]
fn test_simple_reexport_preserved() {
    let files = &[
        ("mod.wj", "pub mod math\n"),
        (
            "math/mod.wj",
            "pub mod vec2\npub mod vec3\npub use vec2::Vec2\npub use vec3::Vec3\n",
        ),
        (
            "math/vec2.wj",
            "pub struct Vec2 { pub x: f64, pub y: f64 }\n",
        ),
        (
            "math/vec3.wj",
            "pub struct Vec3 { pub x: f64, pub y: f64, pub z: f64 }\n",
        ),
    ];

    let output = compile_project(files);

    let mod_rs = output
        .get("math/mod.rs")
        .expect("Should generate math/mod.rs");

    println!("Generated math/mod.rs:\n{}", mod_rs);

    // Must have pub mod declarations
    assert!(mod_rs.contains("pub mod vec2;"), "Should have pub mod vec2");
    assert!(mod_rs.contains("pub mod vec3;"), "Should have pub mod vec3");

    // CRITICAL: Must have pub use re-exports from mod.wj
    assert!(
        mod_rs.contains("pub use") && mod_rs.contains("Vec2"),
        "mod.rs must contain pub use Vec2 re-export. Got:\n{}",
        mod_rs
    );
    assert!(
        mod_rs.contains("pub use") && mod_rs.contains("Vec3"),
        "mod.rs must contain pub use Vec3 re-export. Got:\n{}",
        mod_rs
    );
}

/// Wildcard re-export: pub use utils::* in mod.wj
#[test]
fn test_wildcard_reexport_preserved() {
    let files = &[
        ("mod.wj", "pub mod mymod\n"),
        (
            "mymod/mod.wj",
            "pub mod utils\npub mod types\npub use utils::*\n",
        ),
        (
            "mymod/utils.wj",
            "pub fn helper() -> i32 { 42 }\n",
        ),
        (
            "mymod/types.wj",
            "pub struct MyType { pub x: i32 }\n",
        ),
    ];

    let output = compile_project(files);

    let mod_rs = output
        .get("mymod/mod.rs")
        .expect("Should generate mymod/mod.rs");

    assert!(
        mod_rs.contains("pub use") && mod_rs.contains("utils"),
        "mod.rs must contain pub use utils re-export. Got:\n{}",
        mod_rs
    );
}

/// Multiple re-exports from same module
#[test]
fn test_multiple_reexports_from_same_module() {
    let files = &[
        ("mod.wj", "pub mod math\n"),
        (
            "math/mod.wj",
            "pub mod vec2\npub use vec2::Vec2\npub use vec2::vec2_add\n",
        ),
        (
            "math/vec2.wj",
            "pub struct Vec2 { pub x: f64, pub y: f64 }\npub fn vec2_add(a: Vec2, b: Vec2) -> Vec2 { a }\n",
        ),
    ];

    let output = compile_project(files);

    let mod_rs = output
        .get("math/mod.rs")
        .expect("Should generate math/mod.rs");

    assert!(
        mod_rs.contains("Vec2"),
        "Must re-export Vec2. Got:\n{}",
        mod_rs
    );
    assert!(
        mod_rs.contains("vec2_add"),
        "Must re-export vec2_add. Got:\n{}",
        mod_rs
    );
}
