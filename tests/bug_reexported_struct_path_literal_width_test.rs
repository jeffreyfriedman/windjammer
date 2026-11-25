#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

//! A struct literal written through a re-export (`ffi::Vertex`, the struct declared in `ffi::api`) types its float
//! literals from the struct's field types: `[0.0, 0.0]` in a `[f32; 2]` field is `f32`. The IR solver looked the
//! written name up directly, found no field types, and gave the literals its default `f64` (rustc E0308).

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use tempfile::TempDir;

const API: &str = "pub struct Vertex { pub position: [f32; 3], pub uv: [f32; 2] }\n";
const USER: &str = r#"use crate::ffi

pub fn make(x: f32) -> ffi::Vertex {
    ffi::Vertex { position: [x, 0.0, 0.0], uv: [0.0, 1.0] }
}
"#;

fn build_user_rs(ffi_mod: &str) -> String {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    let out = tmp.path().join("out");
    fs::create_dir_all(src.join("ffi")).unwrap();
    fs::write(src.join("ffi/api.wj"), API).unwrap();
    fs::write(src.join("ffi/mod.wj"), ffi_mod).unwrap();
    fs::write(src.join("user.wj"), USER).unwrap();
    let output = test_utils::run_wj_command([
        "build",
        src.to_str().unwrap(),
        "--library",
        "-o",
        out.to_str().unwrap(),
        "--no-cargo",
        "--no-generate-cargo-toml",
        "--module-file",
    ]);
    assert!(output.status.success(), "wj build failed:\n{}", String::from_utf8_lossy(&output.stderr));
    fs::read_to_string(out.join("user.rs")).expect("user.rs")
}

fn assert_f32_literals(generated: &str) {
    assert!(
        generated.contains("[0.0_f32, 1.0_f32]"),
        "the [f32; 2] field's literals are f32. Got:\n{generated}"
    );
    assert!(!generated.contains("_f64"), "no f64 literal in an all-f32 struct. Got:\n{generated}");
}

#[test]
fn test_struct_literal_through_explicit_pub_use_types_f32_array_field() {
    assert_f32_literals(&build_user_rs("pub mod api\npub use api::Vertex\n"));
}

#[test]
fn test_struct_literal_through_implicit_child_reexport_types_f32_array_field() {
    assert_f32_literals(&build_user_rs("pub mod api\n"));
}
