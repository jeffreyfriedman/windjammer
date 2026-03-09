// TDD Test: Float literal inference in match expression assigned to variable
//
// Bug: let x = match map.get(k) { None => 999999.0 } doesn't infer from map type
// Pattern: Variable assignment from match, need to track HashMap<K, f32> → f32
//
// Dogfooding Win: Exact pattern from astar_grid.wj line 209

use std::fs;
use std::process::Command;

#[test]
fn test_match_arm_in_variable_assignment() {
    let wj_source = r#"
use std::collections::HashMap

fn find_path(g_score: HashMap<(i32, i32), f32>, x: i32, y: i32) -> f32 {
    let current_g = match g_score.get(&(x, y)) {
        Some(v) => *v,
        None => 999999.0
    }
    current_g
}
"#;

    let output_dir = "/tmp/wj_test_match_let";
    fs::create_dir_all(output_dir).unwrap();
    fs::write(format!("{}/test.wj", output_dir), wj_source).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--release",
            "--",
            "build",
            "--target",
            "rust",
            "--no-cargo",
            &format!("{}/test.wj", output_dir),
            "--output",
            output_dir,
        ])
        .current_dir("/Users/jeffreyfriedman/src/wj/windjammer")
        .output()
        .expect("Failed to run wj");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Compilation should succeed, stderr: {}",
        stderr
    );

    let rust_code = fs::read_to_string(format!("{}/test.rs", output_dir))
        .expect("Generated Rust file not found");

    eprintln!("Generated Rust:\n{}", rust_code);

    // The literal 999999.0 should be f32 (from HashMap<K, f32> → f32)
    assert!(
        !rust_code.contains("999999.0_f64") && !rust_code.contains("999999_f64"),
        "999999.0 should NOT be f64 when HashMap value type is f32, got:\n{}",
        rust_code
    );
}
