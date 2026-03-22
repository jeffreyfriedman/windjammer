// TDD Test: Float literal inference in match arms
//
// Bug: Match arms returning float literals don't constrain to expected type
// Pattern: match option { Some(x) => x, None => 999999.0 } // Should be 999999.0_f32
//
// Dogfooding Win: Common pattern in game code (default values)

use std::fs;
use std::process::Command;

#[test]
fn test_float_literal_in_match_arm() {
    let wj_source = r#"
fn get_score_or_default(scores: HashMap<i32, f32>, key: i32) -> f32 {
    match scores.get(key) {
        Some(score) => *score,
        None => 999999.0
    }
}
"#;

    let output_dir = "/tmp/wj_test_match_arm";
    fs::create_dir_all(output_dir).unwrap();
    fs::write(format!("{}/test.wj", output_dir), wj_source).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj")).args([
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

    // The literal 999999.0 should be f32 (from function return type)
    assert!(
        !rust_code.contains("999999.0_f64") && !rust_code.contains("999999_f64"),
        "999999.0 should NOT be f64 when match arm returns f32, got:\n{}",
        rust_code
    );
}
