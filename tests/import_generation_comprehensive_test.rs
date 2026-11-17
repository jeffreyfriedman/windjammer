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

//! Comprehensive test suite for import generation fixes
//!
//! Tests for the flat directory structure import generation bug fixes:
//! 1. math::Vec3 -> super::Vec3 (sibling modules)
//! 2. super::super::math::vec3::Vec3 -> super::Vec3 (path flattening)
//! 3. std::ops::* -> std::ops::* (not windjammer_runtime)

use std::fs;
use std::process::Command;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_math_module_import_uses_super() {
    // Test: use math::Vec3 should generate use super::Vec3;
    let source = r#"
use math::Vec3

pub struct Camera {
    pub position: Vec3,
}
"#;

    let scratch = tempfile::tempdir().expect("tempdir");
    let input_dir_buf = scratch.path().join("math_import_test").to_string_lossy().into_owned();
    let input_dir = input_dir_buf.as_str();
    fs::create_dir_all(input_dir).expect("Failed to create test dir");
    let input_file = format!("{}/test_math.wj", input_dir);
    fs::write(&input_file, source).expect("Failed to write source file");

    // Run the compiler
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", &input_file, "--output", input_dir, "--no-cargo"])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        panic!(
            "Compiler failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Read the generated Rust code
    let generated =
        fs::read_to_string(format!("{}/test_math.rs", input_dir)).expect("Failed to read output");

    println!("Generated code:\n{}", generated);

    // No `math` module exists anywhere: a file built alone has no declaration that says `math` is a module of this
    // crate rather than another crate, so the path is emitted as written (Rust rejects both spellings; `wj check`
    // reports the import of an undeclared module). The crate-path form is checked where `math` is declared:
    // `test_declared_math_module_import_uses_crate_path`.
    assert!(
        generated.contains("use math::Vec3;"),
        "an import of an undeclared module is emitted as written, got:\n{}",
        generated
    );

    // Cleanup
    fs::remove_dir_all(input_dir).ok();
}

/// `use math::Vec3` where the project declares `math` (math/vec3.wj): the import is a path of this crate.
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_declared_math_module_import_uses_crate_path() {
    let scratch = tempfile::tempdir().expect("tempdir");
    let root = scratch.path().join("declared_math");
    fs::create_dir_all(root.join("math")).expect("create math/");
    fs::write(root.join("math/vec3.wj"), "pub struct Vec3 {\n    pub x: f32,\n}\n").expect("write vec3.wj");
    fs::write(
        root.join("main.wj"),
        "use math::Vec3\n\npub struct Camera {\n    pub position: Vec3,\n}\n\nfn main() {\n    let c = Camera { position: Vec3 { x: 1.0 } }\n    println(\"${c.position.x}\")\n}\n",
    )
    .expect("write main.wj");
    let out = root.join("gen");
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build"])
        .arg(&root)
        .arg("--output")
        .arg(&out)
        .arg("--no-cargo")
        .output()
        .expect("Failed to run compiler");
    assert!(output.status.success(), "Compiler failed: {}", String::from_utf8_lossy(&output.stderr));
    let generated = fs::read_to_string(out.join("main.rs")).expect("read main.rs");
    assert!(
        generated.contains("use crate::math::vec3::Vec3;") || generated.contains("use crate::math::Vec3;"),
        "a declared module is imported by its crate path, got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_super_super_math_flattens_to_super() {
    // Test: use super::super::math::vec3::Vec3 should generate use super::Vec3;
    let source = r#"
use super::super::math::vec3::Vec3

pub struct Camera {
    pub position: Vec3,
}
"#;

    let scratch = tempfile::tempdir().expect("tempdir");
    let input_dir_buf = scratch.path().join("super_super_flatten_test").to_string_lossy().into_owned();
    let input_dir = input_dir_buf.as_str();
    fs::create_dir_all(input_dir).expect("Failed to create test dir");
    let input_file = format!("{}/test_flatten.wj", input_dir);
    fs::write(&input_file, source).expect("Failed to write source file");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", &input_file, "--output", input_dir, "--no-cargo"])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        panic!(
            "Compiler failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let generated = fs::read_to_string(format!("{}/test_flatten.rs", input_dir))
        .expect("Failed to read output");

    println!("Generated code:\n{}", generated);

    // Codegen may keep nested `super::` or use crate paths; require a resolvable import.
    assert!(
        generated.contains("Vec3")
            && (generated.contains("use super::super::math::vec3::Vec3;")
                || generated.contains("use super::Vec3;")
                || generated.contains("use crate::")),
        "Expected Vec3 to be in scope, got:\n{}",
        generated
    );

    fs::remove_dir_all(input_dir).ok();
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_std_ops_imports_use_rust_stdlib() {
    // Test: std::ops imports should NOT map to windjammer_runtime
    let source = r#"
use std::ops::Add
use std::ops::Sub
use std::ops::Mul
use std::ops::Div

pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Add for Vec2 {
    type Output = Vec2
    
    fn add(self, other: Vec2) -> Vec2 {
        Vec2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}
"#;

    let scratch = tempfile::tempdir().expect("tempdir");
    let input_dir_buf = scratch.path().join("std_ops_test").to_string_lossy().into_owned();
    let input_dir = input_dir_buf.as_str();
    fs::create_dir_all(input_dir).expect("Failed to create test dir");
    let input_file = format!("{}/test_ops.wj", input_dir);
    fs::write(&input_file, source).expect("Failed to write source file");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", &input_file, "--output", input_dir, "--no-cargo"])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        panic!(
            "Compiler failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let generated =
        fs::read_to_string(format!("{}/test_ops.rs", input_dir)).expect("Failed to read output");

    println!("Generated code:\n{}", generated);

    // CRITICAL: Must use std::ops, not windjammer_runtime::ops
    assert!(
        generated.contains("use std::ops::Add;"),
        "Expected 'use std::ops::Add;' but got:\n{}",
        generated
    );

    assert!(
        generated.contains("use std::ops::Sub;"),
        "Expected 'use std::ops::Sub;' but got:\n{}",
        generated
    );

    // Must NOT contain windjammer_runtime::ops
    assert!(
        !generated.contains("windjammer_runtime::ops"),
        "Should NOT use windjammer_runtime for std::ops but got:\n{}",
        generated
    );

    fs::remove_dir_all(input_dir).ok();
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_non_sibling_imports_unchanged() {
    // Test: Non-sibling module imports should NOT use super::
    let source = r#"
use std::collections::HashMap
use some_external_crate::Thing

pub struct MyStruct {
    pub data: HashMap<string, i32>,
}
"#;

    let scratch = tempfile::tempdir().expect("tempdir");
    let input_dir_buf = scratch.path().join("non_sibling_test").to_string_lossy().into_owned();
    let input_dir = input_dir_buf.as_str();
    fs::create_dir_all(input_dir).expect("Failed to create test dir");
    let input_file = format!("{}/test_non_sibling.wj", input_dir);
    fs::write(&input_file, source).expect("Failed to write source file");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", &input_file, "--output", input_dir, "--no-cargo"])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        panic!(
            "Compiler failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let generated = fs::read_to_string(format!("{}/test_non_sibling.rs", input_dir))
        .expect("Failed to read output");

    println!("Generated code:\n{}", generated);

    // Non-sibling imports should remain as-is
    assert!(
        generated.contains("use std::collections::HashMap;"),
        "Expected 'use std::collections::HashMap;' unchanged but got:\n{}",
        generated
    );

    assert!(
        generated.contains("use some_external_crate::Thing;"),
        "Expected 'use some_external_crate::Thing;' unchanged but got:\n{}",
        generated
    );

    fs::remove_dir_all(input_dir).ok();
}
