// TDD TEST: String literals should work with borrowed string parameters
//
// PHASE 1 BASELINE: greet(name: string) with borrowed ownership generates
// greet(name: &String), and string literals need conversion: "World" → &"World".to_string()

use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_string_literal_to_borrowed_string_param() {
    let temp_dir = std::env::temp_dir().join("wj_test_string_literal");
    fs::create_dir_all(&temp_dir).unwrap();

    let wj_code = r#"
pub fn greet(name: string) -> string {
    format!("Hello, {}!", name)
}

pub fn test_greet() -> string {
    greet("World")
}
"#;

    let wj_file = temp_dir.join("test.wj");
    fs::write(&wj_file, wj_code).unwrap();

    // Find wj compiler (use absolute path)
    let wj_bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");

    // Compile with wj
    let output = Command::new(&wj_bin)
        .args(&["build", "test.wj", "--no-cargo"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to run wj compiler");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    println!("=== WJ COMPILE OUTPUT ===");
    println!("stdout: {}", stdout);
    println!("stderr: {}", stderr);

    // Read generated Rust code
    let rs_file = temp_dir.join("build").join("test.rs");
    let rust_code = fs::read_to_string(&rs_file).expect("Failed to read generated Rust file");

    println!("=== GENERATED RUST ===");
    println!("{}", rust_code);

    // TDD ASSERTION: Verify rustc compiles without E0308
    let rustc_output = Command::new("rustc")
        .args(&[
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            rs_file.to_str().unwrap(),
            "--out-dir",
            temp_dir.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run rustc");

    let rustc_stderr = String::from_utf8_lossy(&rustc_output.stderr);
    println!("=== RUSTC OUTPUT ===");
    println!("{}", rustc_stderr);

    // PHASE 1 BASELINE: Check that borrowed string param generates &String
    assert!(
        rust_code.contains("fn greet(name: &String)"),
        "FAIL: Should generate &String parameter (Phase 1 baseline)!\n\
         Generated:\n{}",
        rust_code
    );

    // PHASE 1 BASELINE: String literals converted to &"literal".to_string()
    assert!(
        rust_code.contains(r#"greet(&"World".to_string())"#),
        "FAIL: Should convert string literal to &\"literal\".to_string()!\n\
         Generated:\n{}",
        rust_code
    );

    // No E0308 after conversion
    assert!(
        !rustc_stderr.contains("E0308"),
        "FAIL: Generated Rust has E0308 (type mismatch)!\n{}",
        rustc_stderr
    );

    assert!(
        rustc_output.status.success(),
        "Rustc compilation failed:\n{}",
        rustc_stderr
    );

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);
}
