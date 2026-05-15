//! E0432 regression: nested `src/<dir>/*.rs` + auto-generated `use super::...` imports.
//!
//! `rust_use_path_from_module_to_type` already emits the correct number of `super::` segments for
//! Rust's module tree. Prepending filesystem nesting (`src/foo/bar.rs` → depth 1) produced
//! `use super::super::sibling::Type` and broke sibling imports under `rendering/`, etc.

use std::fs;
use tempfile::TempDir;
use windjammer::{build_project_ext, CompilationTarget};

#[test]
fn test_nested_src_subdir_auto_import_no_double_super() {
    let temp = TempDir::new().unwrap();
    let src = temp.path();
    let out = temp.path().join("build");
    let rendering = src.join("rendering");
    fs::create_dir_all(&rendering).unwrap();

    fs::write(
        rendering.join("mod.wj"),
        r#"
pub mod render_port
pub mod consumer
"#,
    )
    .unwrap();

    fs::write(
        rendering.join("render_port.wj"),
        r#"
pub struct CameraData {
    pub x: f32,
}
"#,
    )
    .unwrap();

    // Explicit crate glob disables injected `use super::*` but external types still get auto-uses.
    fs::write(
        rendering.join("consumer.wj"),
        r#"
use crate::rendering::render_port::*

pub struct Holder {
    pub cam: CameraData,
}
"#,
    )
    .unwrap();

    build_project_ext(src, &out, CompilationTarget::Rust, false, true, &[])
        .expect("multipass build");

    let generated = fs::read_to_string(out.join("rendering/consumer.rs")).unwrap();
    assert!(
        generated.contains("use super::render_port::CameraData"),
        "expected single super hop to sibling module; got:\n{}",
        generated
    );
    assert!(
        !generated.contains("use super::super::render_port::CameraData"),
        "must not double-prefix filesystem nesting onto Rust module-relative path; got:\n{}",
        generated
    );
}

#[test]
fn test_crate_input_type_prefers_shorter_submodule_on_tie() {
    let temp = TempDir::new().unwrap();
    let src = temp.path();
    let out = temp.path().join("build");
    let input = src.join("input");
    fs::create_dir_all(&input).unwrap();

    fs::write(
        input.join("mod.wj"),
        r#"
pub mod input
pub mod input_interface
"#,
    )
    .unwrap();

    fs::write(
        input.join("input_interface.wj"),
        r#"
pub struct Input {
    pub tag: i32,
}
"#,
    )
    .unwrap();

    fs::write(
        input.join("input.wj"),
        r#"
pub struct Input {
    pub tag: i32,
}
"#,
    )
    .unwrap();

    fs::write(
        input.join("prelude.wj"),
        r#"
pub use crate::input::Input
"#,
    )
    .unwrap();

    build_project_ext(src, &out, CompilationTarget::Rust, false, true, &[])
        .expect("multipass build");

    let generated = fs::read_to_string(out.join("input/prelude.rs")).unwrap();
    assert!(
        generated.contains("crate::input::input::Input"),
        "duplicate `Input` defs at same depth: prefer `input::` over longer `input_interface::`; got:\n{}",
        generated
    );
    assert!(
        !generated.contains("input_interface::Input"),
        "should not pick auxiliary module when names tie at same depth; got:\n{}",
        generated
    );
}
