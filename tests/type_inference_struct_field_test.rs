// TDD Test: Float literal inference from struct field types
//
// Bug: AStarCell { cost: 1.0 } generates cost: 1.0_f64 even though cost: f32
// Expected: Struct field type should constrain literal type
//
// Dogfooding Win: This pattern appears hundreds of times in game code

use std::fs;
use std::process::Command;

#[test]
fn test_float_literal_in_struct_field() {
    let wj_source = r#"
struct Cell {
    pub walkable: bool,
    pub cost: f32,
}

fn main() {
    let cell = Cell { walkable: true, cost: 1.0 }
    println!("{}", cell.cost)
}
"#;

    let output_dir = "/tmp/wj_test_struct_field";
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

    // The literal 1.0 in struct field should be f32 (from cost: f32)
    assert!(
        !rust_code.contains("cost: 1.0_f64") && !rust_code.contains("cost: 1_f64"),
        "1.0 should NOT be f64 when assigned to f32 struct field, got:\n{}",
        rust_code
    );

    assert!(
        rust_code.contains("cost: 1.0_f32") || rust_code.contains("cost: 1_f32") || rust_code.contains("cost: 1.0"),
        "1.0 should be f32 (or type-inferred) when assigned to f32 struct field, got:\n{}",
        rust_code
    );
}
