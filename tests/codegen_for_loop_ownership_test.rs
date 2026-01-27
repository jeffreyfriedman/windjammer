/// TDD Test: For loop parameters should be inferred as owned when iterated directly
///
/// Bug: When a Vec parameter is used in `for item in vec`, the compiler incorrectly
/// infers the parameter as `&Vec` instead of owned `Vec`, causing elements to be
/// references when they should be owned.
///
/// Expected: `fn process(items: Vec<String>)` not `fn process(items: &Vec<String>)`
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_vec_param_in_for_loop_stays_owned() {
    let code = r#"
pub fn process_items(items: Vec<string>) {
    for item in items {
        // item should be String (owned), not &String
        let upper = item.to_uppercase()
    }
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path();
    let wj_file = output_dir.join("test.wj");

    fs::write(&wj_file, code).unwrap();

    // Compile
    let result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&wj_file)
        .arg("--output")
        .arg(output_dir)
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj build");

    if !result.status.success() {
        eprintln!(
            "wj build failed:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        panic!("Transpilation should succeed");
    }

    // Read generated code
    let rust_file = output_dir.join("test.rs");
    let rust_code = fs::read_to_string(&rust_file).expect("Should find generated Rust file");

    println!("Generated code:\n{}", rust_code);

    // Should generate: fn process_items(items: Vec<String>)
    // NOT: fn process_items(items: &Vec<String>)
    assert!(
        rust_code.contains("items: Vec<String>"),
        "Parameter should be owned Vec, not borrowed. Generated: {}",
        rust_code
    );
    assert!(
        !rust_code.contains("items: &Vec<String>"),
        "Parameter should NOT be borrowed Vec. Generated: {}",
        rust_code
    );

    // Verify it compiles with rustc
    let compile_result = Command::new("rustc")
        .arg("--crate-type")
        .arg("lib")
        .arg("--edition")
        .arg("2021")
        .arg(&rust_file)
        .arg("-o")
        .arg(output_dir.join("test.rlib"))
        .output()
        .expect("Failed to run rustc");

    if !compile_result.status.success() {
        eprintln!("Generated Rust code:\n{}", rust_code);
        eprintln!(
            "Rust compilation failed:\n{}",
            String::from_utf8_lossy(&compile_result.stderr)
        );
        panic!("Generated code should compile with rustc");
    }
}

#[test]
fn test_vec_tuple_param_in_for_loop_stays_owned() {
    let code = r#"
pub fn process_pairs(pairs: Vec<(string, string)>) {
    for (first, second) in pairs {
        // first and second should be String (owned), not &String
        let combined = format!("{} {}", first, second)
    }
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path();
    let wj_file = output_dir.join("test.wj");

    fs::write(&wj_file, code).unwrap();

    // Compile
    let result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&wj_file)
        .arg("--output")
        .arg(output_dir)
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj build");

    if !result.status.success() {
        eprintln!(
            "wj build failed:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        panic!("Transpilation should succeed");
    }

    // Read generated code
    let rust_file = output_dir.join("test.rs");
    let rust_code = fs::read_to_string(&rust_file).expect("Should find generated Rust file");

    println!("Generated code:\n{}", rust_code);

    // Should generate: fn process_pairs(pairs: Vec<(String, String)>)
    // NOT: fn process_pairs(pairs: &Vec<(String, String)>)
    assert!(
        rust_code.contains("pairs: Vec<(String, String)>"),
        "Parameter should be owned Vec, not borrowed. Generated: {}",
        rust_code
    );
    assert!(
        !rust_code.contains("pairs: &Vec<(String, String)>"),
        "Parameter should NOT be borrowed Vec. Generated: {}",
        rust_code
    );

    // Verify it compiles with rustc
    let compile_result = Command::new("rustc")
        .arg("--crate-type")
        .arg("lib")
        .arg("--edition")
        .arg("2021")
        .arg(&rust_file)
        .arg("-o")
        .arg(output_dir.join("test.rlib"))
        .output()
        .expect("Failed to run rustc");

    if !compile_result.status.success() {
        eprintln!("Generated Rust code:\n{}", rust_code);
        eprintln!(
            "Rust compilation failed:\n{}",
            String::from_utf8_lossy(&compile_result.stderr)
        );
        panic!("Generated code should compile with rustc");
    }
}
