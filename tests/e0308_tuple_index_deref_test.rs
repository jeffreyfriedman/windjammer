//! E0308 Fix: let (nx, ny, move_cost) = neighbors[ni]
//!
//! Vec index returns &T. Destructuring &(i32, i32, f32) gives &i32, &i32, &f32.
//! For Copy tuple, emit * to get owned values so nx, ny are i32 not &i32.

use std::path::PathBuf;
use std::process::Command;

fn compile_and_get_rust(source: &str) -> String {
    let test_dir = std::env::temp_dir().join("wj_e0308_tuple_test");
    let _ = std::fs::create_dir_all(&test_dir);
    let input = test_dir.join("test.wj");
    std::fs::write(&input, source).expect("write test file");

    let wj_bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");
    let _ = Command::new(&wj_bin)
        .arg("build")
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .arg(&input)
        .arg("--output")
        .arg(&test_dir)
        .output()
        .expect("wj build");

    let rust_file = test_dir.join("test.rs");
    std::fs::read_to_string(&rust_file).unwrap_or_else(|_| String::new())
}

#[test]
fn test_tuple_from_index_gets_deref_for_copy() {
    let source = r#"
use std::collections::HashMap

pub fn pathfind() -> i32 {
    let mut g_score: HashMap<(i32, i32), f32> = HashMap::new()
    g_score.insert((0, 0), 0.0)
    let neighbors: Vec<(i32, i32, f32)> = Vec::new()
    let mut ni = 0
    while ni < neighbors.len() {
        let (nx, ny, move_cost) = neighbors[ni]
        let _ = g_score.get(&(nx, ny))
        ni = ni + 1
    }
    0
}
"#;

    let rust = compile_and_get_rust(source);

    // Should generate *neighbors[ni] so nx, ny are i32 (not &i32)
    assert!(
        rust.contains("*neighbors[ni]") || rust.contains("* neighbors[ni]"),
        "Expected * for tuple-from-index Copy destructuring, got:\n{}",
        rust
    );
}
