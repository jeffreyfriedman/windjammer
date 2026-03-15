//! TDD Test: Float literal inference in match arms (astar_grid pattern)
//!
//! Bug: `match g_score.get(&(x, y)) { Some(v) => *v, None => 999999.0 }` generates
//! 999999.0_f64 when *v is f32, causing E0308 "expected f32, found f64".
//!
//! Root cause: get_known_float_type_from_expr didn't handle Unary Deref (*v),
//! and match pattern variables (v) weren't populated for Some(v) over Option<&f32>.

use std::fs;
use std::process::Command;

fn compile_and_get_rust(source: &str) -> String {
    let output_dir = "/tmp/wj_test_astar_match";
    fs::create_dir_all(output_dir).unwrap();
    fs::write(format!("{}/test.wj", output_dir), source).unwrap();

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

    fs::read_to_string(format!("{}/test.rs", output_dir))
        .expect("Generated Rust file not found")
}

#[test]
fn test_match_hashmap_get_none_arm_infers_f32() {
    let wj_source = r#"
use std::collections::HashMap

pub fn pathfind(grid: (i32, i32), start: (i32, i32), goal: (i32, i32)) -> f32 {
    let mut g_score: HashMap<(i32, i32), f32> = HashMap::new()
    g_score.insert((0, 0), 0.0)
    let (current_x, current_y) = (0, 0)
    let current_g = match g_score.get(&(current_x, current_y)) {
        Some(v) => *v,
        None => 999999.0,
    }
    current_g
}
"#;

    let rust_code = compile_and_get_rust(wj_source);

    // The literal 999999.0 should be f32 (from *v in other arm)
    assert!(
        rust_code.contains("999999.0_f32") || rust_code.contains("999999.0f32"),
        "999999.0 should be f32 when match arm has *v from HashMap<_, f32>.get(), got:\n{}",
        rust_code
    );
    assert!(
        !rust_code.contains("999999.0_f64") && !rust_code.contains("999999_f64"),
        "Should NOT use f64 when other arm is f32, got:\n{}",
        rust_code
    );
}
