// TDD Test: Float literal inference in Vec.push()
//
// Bug: scores.push(0.0) generates 0.0_f64 for Vec<f32>
// Expected: Vec<f32> → push(f32) should constrain argument
//
// Dogfooding Win: Common pattern in game code

use std::fs;
use std::process::Command;

#[test]
fn test_vec_push_float_literal() {
    let wj_source = r#"
fn init_scores() -> Vec<f32> {
    let mut scores: Vec<f32> = Vec::new()
    scores.push(0.0)
    scores.push(1.0)
    scores.push(2.5)
    scores
}
"#;

    let output_dir = "/tmp/wj_test_vec_push";
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

    // All literals should be f32 (from Vec<f32> → push(f32))
    assert!(
        !rust_code.contains("_f64"),
        "Float literals should NOT be f64 when pushing to Vec<f32>, got:\n{}",
        rust_code
    );
}
