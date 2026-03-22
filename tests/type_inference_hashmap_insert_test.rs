// TDD Test: Float literal inference in HashMap.insert()
//
// Bug: g_score.insert((x, y), 0.0) generates 0.0_f64 for HashMap<K, f32>
// Expected: HashMap<K, f32> → insert() value param should be f32
//
// Dogfooding Win: Hundreds of HashMap operations in game code

use std::fs;
use std::process::Command;

#[test]
fn test_hashmap_insert_float_literal() {
    let wj_source = r#"
use std::collections::HashMap

fn init_scores() -> HashMap<(i32, i32), f32> {
    let mut g_score: HashMap<(i32, i32), f32> = HashMap::new()
    g_score.insert((0, 0), 0.0)
    g_score
}
"#;

    let output_dir = "/tmp/wj_test_hashmap_insert";
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

    // The literal 0.0 should be f32 (from HashMap<K, f32> → insert(K, f32))
    assert!(
        !rust_code.contains("0.0_f64") && !rust_code.contains("0_f64"),
        "0.0 should NOT be f64 when inserting into HashMap<K, f32>, got:\n{}",
        rust_code
    );
}
