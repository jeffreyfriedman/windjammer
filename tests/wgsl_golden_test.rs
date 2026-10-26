#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "parser_tests",
))]

//! Characterization (golden) tests for every WGSL entry point:
//!   * `wjsl::transpile_wjsl`            (RFC-syntax .wjsl fixtures)
//!   * `wjsl::compile_legacy_shader`     (block-syntax `shader Name { ... }`, `wj shader-compile`)
//!   * `wj build --target wgsl`          (Windjammer source -> WGSL backend)
//!
//! Goldens live in `tests/golden/wgsl/`. They pin the exact output so the
//! shader front-end/printer consolidation is provably behavior-preserving.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn golden_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/wgsl")
        .join(format!("{name}.wgsl"))
}

fn assert_golden(name: &str, actual: &str) {
    let path = golden_path(name);
    let expected = fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing golden {}", path.display()));
    assert_eq!(expected, actual, "golden mismatch for {name}");
}

// ---------------------------------------------------------------------------
// RFC-syntax WJSL fixtures
// ---------------------------------------------------------------------------

fn wjsl_corpus() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files: Vec<PathBuf> = fs::read_dir(root.join("tests/fixtures/shaders"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "wjsl"))
        .collect();
    files.sort();
    files
}

#[test]
fn golden_wjsl_fixture_corpus() {
    let files = wjsl_corpus();
    assert!(files.len() > 30, "corpus unexpectedly small");
    for file in files {
        let source = fs::read_to_string(&file).unwrap();
        let base = file.parent().unwrap();
        let actual = windjammer::wjsl::transpile_wjsl_with_includes(&source, base)
            .unwrap_or_else(|e| format!("ERROR: {e}"));
        let stem = file.file_stem().unwrap().to_string_lossy().to_string();
        assert_golden(&format!("wjsl_{stem}"), &actual);
    }
}

// ---------------------------------------------------------------------------
// Block-syntax shaders (`wj shader-compile`)
// ---------------------------------------------------------------------------

const LEGACY_CORPUS: &[(&str, &str)] = &[
    (
        "simple",
        "shader MyShader {\n uniform screen_size: Vec2<f32>\n storage output: array<Vec4<f32>>\n}",
    ),
    ("empty", "shader Empty { }"),
    (
        "vectors",
        "shader S { uniform a: Vec2<f32> uniform b: Vec3<u32> uniform c: Vec4<i32> uniform m: Mat4 }",
    ),
    (
        "storage_modes",
        "shader S { storage a: array<f32> storage b: array<Vec2<f32>> }",
    ),
];

#[test]
fn golden_legacy_block_shaders() {
    for (name, source) in LEGACY_CORPUS {
        let actual = windjammer::wjsl::compile_legacy_shader(source)
            .unwrap_or_else(|e| format!("ERROR: {e}"));
        assert_golden(&format!("legacy_{name}"), &actual);
    }
}

#[test]
fn golden_legacy_block_shader_rejects_f64() {
    assert!(windjammer::wjsl::compile_legacy_shader("shader S { uniform x: Vec2<f64> }").is_err());
}

// ---------------------------------------------------------------------------
// Windjammer source -> WGSL backend (`wj build --target wgsl`)
// ---------------------------------------------------------------------------

const BACKEND_CORPUS: &[(&str, &str)] = &[
    (
        "add",
        "pub fn add(x: uint, y: uint) -> uint {\n    x + y\n}\n",
    ),
    (
        "struct_layout",
        "pub struct Layout1 {\n    count: uint,\n    position: vec2<float>,\n    dir: vec3<float>,\n    tint: vec4<float>,\n}\n",
    ),
    (
        "uniform_safe",
        "@uniform\n@binding(3)\nextern let screen_size: vec2<uint>;\n\n@compute(workgroup_size = [8, 8, 1])\nfn main(@builtin(global_invocation_id) id: vec3<uint>) {\n    let w = screen_size.x as uint;\n}\n",
    ),
    (
        "vertex_fragment",
        "pub struct VertexOutput {\n    position: vec4<float>,\n    color: vec4<float>,\n}\n\n@vertex\npub fn vs_main(@location(0) pos: vec2<float>) -> VertexOutput {\n    VertexOutput {\n        position: vec4(pos.x, pos.y, 0.0, 1.0),\n        color: vec4(1.0, 1.0, 1.0, 1.0),\n    }\n}\n\n@fragment\npub fn fs_main(@location(0) color: vec4<float>) -> @location(0) vec4<float> {\n    color\n}\n",
    ),
    (
        "mutable_locals",
        "pub fn counter() -> uint {\n    let mut count = 0;\n    count = count + 1;\n    count\n}\n\n@compute(workgroup_size = [8, 8, 1])\npub fn update(@builtin(global_invocation_id) id: vec3<uint>) {\n    let mut data: [uint; 10] = [0; 10];\n    data[id.x] = 42\n}\n",
    ),
    (
        "control_flow",
        "pub fn pick(a: int32, b: int32) -> int32 {\n    if a > b {\n        return a\n    }\n    let mut i = 0;\n    while i < 4 {\n        i = i + 1;\n    }\n    b\n}\n",
    ),
];

fn build_wgsl(source: &str) -> String {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("test.wj");
    fs::write(&input, source).unwrap();
    let out_dir = dir.path().join("out");
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build"])
        .arg(&input)
        .args(["--target", "wgsl", "--no-cargo", "--output"])
        .arg(&out_dir)
        .output()
        .expect("run wj");
    if !output.status.success() {
        return format!(
            "ERROR: {}",
            String::from_utf8_lossy(&output.stderr)
                .lines()
                .filter(|l| !l.trim().is_empty())
                .last()
                .unwrap_or("")
        );
    }
    fs::read_to_string(out_dir.join("test.wgsl")).expect("generated wgsl")
}

#[test]
fn golden_wgsl_backend_corpus() {
    for (name, source) in BACKEND_CORPUS {
        assert_golden(&format!("backend_{name}"), &build_wgsl(source));
    }
}
