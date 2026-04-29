// TDD TEST: String literals should work with borrowed string parameters
//
// PHASE 1 BASELINE: greet(name: string) with borrowed ownership generates
// greet(name: &String), and string literals need conversion: "World" → &"World".to_string()

use std::fs;
use std::process::Command;

#[test]
fn test_string_literal_to_borrowed_string_param() {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let out_dir = temp_dir.path().join("out");
    fs::create_dir_all(&out_dir).unwrap();

    let wj_code = r#"
pub fn greet(name: string) -> string {
    format!("Hello, {}!", name)
}

pub fn test_greet() -> string {
    greet("World")
}
"#;

    let wj_file = temp_dir.path().join("test.wj");
    fs::write(&wj_file, wj_code).unwrap();

    let wj_bin = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj_bin)
        .args([
            "build",
            wj_file.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    println!("=== WJ COMPILE OUTPUT ===");
    println!("stdout: {}", stdout);
    println!("stderr: {}", stderr);

    let rs_file = out_dir.join("test.rs");
    let rust_code = fs::read_to_string(&rs_file).expect("Failed to read generated Rust file");

    println!("=== GENERATED RUST ===");
    println!("{}", rust_code);

    // TDD ASSERTION: Verify rustc compiles without E0308
    let rustc_output = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            rs_file.to_str().unwrap(),
            "--out-dir",
            temp_dir.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run rustc");

    let rustc_stderr = String::from_utf8_lossy(&rustc_output.stderr);
    println!("=== RUSTC OUTPUT ===");
    println!("{}", rustc_stderr);

    // Read-only `string` param: idiomatic &str; literals pass as &str (including "World")
    assert!(
        rust_code.contains("fn greet(name: &str)"),
        "FAIL: Should generate &str for borrowed string param!\n\
         Generated:\n{}",
        rust_code
    );

    assert!(
        rust_code.contains(r#"greet("World")"#),
        "FAIL: String literal should call greet with a str literal at the call site!\n\
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
}
