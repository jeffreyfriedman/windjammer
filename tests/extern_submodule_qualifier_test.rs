//! TDD: `parent::symbol` in generated Rust when symbol lives in `parent/child/*.wj` (FFI layout).

use std::fs;
use std::path::Path;
use tempfile::TempDir;
use windjammer::{build_project_ext, CompilationTarget};

#[test]
fn test_ffi_parent_child_extern_fn_qualifies_to_api_submodule() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    fs::create_dir_all(src.join("ffi")).unwrap();

    fs::write(src.join("ffi").join("mod.wj"), "pub mod api\n").unwrap();

    fs::write(
        src.join("ffi").join("api.wj"),
        r#"
extern fn tilemap_check_collision(tilemap_id: u32, x: f32, y: f32, width: f32, height: f32, tile_size: f32) -> bool
"#,
    )
    .unwrap();

    fs::write(
        src.join("usage.wj"),
        r#"
use crate::ffi

pub fn check() -> bool {
    unsafe { ffi::tilemap_check_collision(0_u32, 0.0, 0.0, 1.0, 1.0, 16.0) }
}
"#,
    )
    .unwrap();

    build_project_ext(&src, &build, CompilationTarget::Rust, false, true, &[])
        .expect("multipass build should succeed");

    let usage = fs::read_to_string(build.join("usage.rs")).expect("usage.rs");
    assert!(
        usage.contains("ffi::api::tilemap_check_collision"),
        "expected qualified FFI path ffi::api::..., got:\n{}",
        usage
    );
}

#[test]
fn test_ffi_qualifier_works_when_project_root_is_relative_path() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build_rel");
    fs::create_dir_all(src.join("ffi")).unwrap();

    fs::write(src.join("ffi").join("mod.wj"), "pub mod api\n").unwrap();

    fs::write(
        src.join("ffi").join("api.wj"),
        r#"
extern fn tilemap_check_collision(tilemap_id: u32, x: f32, y: f32, width: f32, height: f32, tile_size: f32) -> bool
"#,
    )
    .unwrap();

    fs::write(
        src.join("usage.wj"),
        r#"
use crate::ffi

pub fn check() -> bool {
    unsafe { ffi::tilemap_check_collision(0_u32, 0.0, 0.0, 1.0, 1.0, 16.0) }
}
"#,
    )
    .unwrap();

    let old = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(temp.path()).expect("chdir temp");
    let outcome = build_project_ext(
        Path::new("src"),
        &build,
        CompilationTarget::Rust,
        false,
        true,
        &[],
    );
    std::env::set_current_dir(old).expect("restore cwd");
    outcome.expect("multipass build with relative src/");

    let usage = fs::read_to_string(build.join("usage.rs")).expect("usage.rs");
    assert!(
        usage.contains("ffi::api::tilemap_check_collision"),
        "relative project root must still populate extern_submodule_qualifiers; got:\n{}",
        usage
    );
}

#[test]
fn test_vec_of_ffi_struct_qualifies_nested_module_in_return_type() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    fs::create_dir_all(src.join("ffi")).unwrap();

    fs::write(src.join("ffi/mod.wj"), "pub mod api\n").unwrap();
    fs::write(
        src.join("ffi/api.wj"),
        r#"
pub struct GpuVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
}
"#,
    )
    .unwrap();

    fs::write(
        src.join("render.wj"),
        r#"
use crate::ffi

pub fn vertices() -> Vec<ffi::GpuVertex> {
    Vec::new()
}
"#,
    )
    .unwrap();

    build_project_ext(&src, &build, CompilationTarget::Rust, false, true, &[])
        .expect("multipass build should succeed");

    let render = fs::read_to_string(build.join("render.rs")).expect("render.rs");
    assert!(
        render.contains("Vec<ffi::api::GpuVertex>"),
        "expected Vec<ffi::api::GpuVertex> after submodule qualification; got:\n{}",
        render
    );
}

#[test]
fn test_qualified_struct_literal_qualifies_nested_ffi_module() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    fs::create_dir_all(src.join("ffi")).unwrap();

    fs::write(src.join("ffi/mod.wj"), "pub mod api\n").unwrap();
    fs::write(
        src.join("ffi/api.wj"),
        r#"
pub struct GpuVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
}
"#,
    )
    .unwrap();

    fs::write(
        src.join("mesh.wj"),
        r#"
use crate::ffi

pub fn one_vertex() -> ffi::GpuVertex {
    ffi::GpuVertex {
        position: [0.0, 0.0, 0.0],
        normal: [0.0, 0.0, 1.0],
        color: [1.0, 1.0, 1.0, 1.0],
    }
}
"#,
    )
    .unwrap();

    build_project_ext(&src, &build, CompilationTarget::Rust, false, true, &[])
        .expect("multipass build should succeed");

    let mesh = fs::read_to_string(build.join("mesh.rs")).expect("mesh.rs");
    assert!(
        mesh.contains("ffi::api::GpuVertex"),
        "expected struct literal + return type to use ffi::api::GpuVertex; got:\n{}",
        mesh
    );
}

#[test]
fn test_impl_uniform_t_emits_impl_t_uniform_t() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    fs::create_dir_all(&src).unwrap();

    fs::write(
        src.join("gpu_types.wj"),
        r#"
pub struct Uniform<T> {
    buffer_id: u32,
    data: T,
}

impl Uniform<T> {
    pub fn new(data: T) -> Uniform<T> {
        Uniform { buffer_id: 0_u32, data: data }
    }
}
"#,
    )
    .unwrap();

    build_project_ext(&src, &build, CompilationTarget::Rust, false, true, &[]).expect("build");

    let out = fs::read_to_string(build.join("gpu_types.rs")).expect("read");
    assert!(
        out.contains("impl<T> Uniform<T>"),
        "Rust requires impl<T> for inherent impl on generic type; got:\n{}",
        out
    );
}
