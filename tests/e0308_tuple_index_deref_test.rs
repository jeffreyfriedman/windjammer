//! Copy tuple from `Vec<(…)>[i]`: Rust yields the tuple by value; no explicit `*` on the index.

use std::process::Command;

fn compile_and_get_rust(source: &str) -> String {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let out_dir = temp_dir.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();
    let input = temp_dir.path().join("test.wj");
    std::fs::write(&input, source).expect("write test file");

    let wj_bin = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj_bin)
        .args([
            "build",
            input.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("wj build");

    let rust_file = out_dir.join("test.rs");
    std::fs::read_to_string(&rust_file).unwrap_or_else(|_| {
        panic!(
            "Generated .rs file not found at {:?}\nstdout: {}\nstderr: {}",
            rust_file,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        )
    })
}

#[test]
fn test_tuple_from_copy_vec_index_no_star() {
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

    assert!(
        !rust.contains("*neighbors[ni]") && !rust.contains("* neighbors[ni]"),
        "must not deref Copy tuple index (E0614). got:\n{}",
        rust
    );
}
