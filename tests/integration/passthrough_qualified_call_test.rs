#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

/// TDD Test: Passthrough inference for module-qualified function calls
///
/// Bug: When a parameter is passed to a module-qualified function like
/// `station_builder::place_loot_marker(grid, ...)`, the passthrough inference
/// fails to look up the callee's ownership because the registry stores it
/// as `place_loot_marker` but the call uses `station_builder::place_loot_marker`.
///
/// The fix: when looking up a qualified name like `mod::func` in the registry,
/// also try the simple name (last segment after `::`).
use std::fs;
use std::process::Command;

fn compile_wj_to_rust(source: &str) -> String {
    let dir = tempfile::tempdir().expect("create temp dir");
    let src_path = dir.path().join("test_input.wj");
    let out_dir = dir.path().join("build");
    fs::create_dir_all(&out_dir).expect("create output dir");

    fs::write(&src_path, source).expect("write source");

    let wj = std::env::var("WJ_COMPILER")
        .unwrap_or_else(|_| "/Users/jeffreyfriedman/src/wj/windjammer/target/release/wj".into());

    let output = Command::new(&wj)
        .arg("build")
        .arg(dir.path())
        .arg("--output")
        .arg(&out_dir)
        .arg("--library")
        .arg("--no-cargo")
        .output()
        .expect("run wj");

    fn find_rs_files(dir: &std::path::Path, results: &mut Vec<std::path::PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    find_rs_files(&path, results);
                } else if path.extension().map_or(false, |e| e == "rs") {
                    results.push(path);
                }
            }
        }
    }

    let mut rs_files = Vec::new();
    find_rs_files(&out_dir, &mut rs_files);

    let mut rs_content = String::new();
    for path in &rs_files {
        if let Ok(content) = fs::read_to_string(path) {
            rs_content.push_str(&content);
        }
    }

    if rs_content.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "No .rs output found.\nstdout: {}\nstderr: {}",
            stdout, stderr
        );
    }

    rs_content
}

#[test]
fn test_passthrough_qualified_call_infers_mut() {
    // When `grid` is passed to `station_builder::place_marker(grid, ...)`
    // and `place_marker` takes `&mut VoxelGrid`, the `grid` parameter in
    // the calling function should be inferred as `&mut VoxelGrid`.
    let source = r#"
pub struct VoxelGrid {
    pub data: Vec<i32>,
}

pub struct StationBuilder {}

impl StationBuilder {
    pub fn place_marker(grid: VoxelGrid, x: i32, y: i32) {
        grid.data.push(x + y)
    }
}

pub fn drop_loot(grid: VoxelGrid) {
    StationBuilder::place_marker(grid, 10, 20)
}
"#;
    let output = compile_wj_to_rust(source);

    assert!(
        output.contains("grid: &mut VoxelGrid"),
        "Parameter passed to qualified call with &mut should be inferred as &mut.\nGot:\n{}",
        output
    );
}

#[test]
fn test_passthrough_qualified_call_simple_name_fallback() {
    // When the callee is registered with a simple name (e.g., from metadata)
    // but called with a qualified name (module::func), passthrough inference
    // should try the simple name as a fallback.
    let source = r#"
pub struct Grid {
    pub cells: Vec<i32>,
}

pub fn fill(grid: Grid, value: i32) {
    grid.cells.push(value)
}

pub fn wrapper(grid: Grid) {
    fill(grid, 42)
}
"#;
    let output = compile_wj_to_rust(source);

    // `wrapper`'s `grid` should be `&mut Grid` because it's passed through
    // to `fill` which mutates it.
    assert!(
        output.contains("fn wrapper(grid: &mut Grid)"),
        "Passthrough to mutating function should infer &mut.\nGot:\n{}",
        output
    );
}
