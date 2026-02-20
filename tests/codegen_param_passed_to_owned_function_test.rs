/// TDD Test: Parameters passed to functions expecting owned values should be owned
///
/// Bug: When a parameter is passed to another function that expects an owned value,
/// the compiler incorrectly infers it as borrowed, causing type mismatches.
///
/// Expected: `fn wrapper(items: Vec<T>)` not `fn wrapper(items: &Vec<T>)`
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_param_passed_to_owned_function_stays_owned() {
    let code = r#"
pub fn process(items: Vec<string>) {
    // This function takes ownership of items
    let count = items.len()
}

pub fn wrapper(items: Vec<string>) {
    // Passing items to process() which expects owned Vec
    process(items)
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

    // process() only reads items (calls .len()), so it should be borrowed
    assert!(
        rust_code.contains("items: &Vec<String>")
            && rust_code
                .lines()
                .any(|l| l.contains("fn process") && l.contains("&Vec<String>")),
        "process() only reads items via .len(), should be borrowed. Generated: {}",
        rust_code
    );
    // wrapper() passes items to process(), which is a consuming argument pass,
    // so items should stay owned in wrapper
    assert!(
        rust_code
            .lines()
            .any(|l| l.contains("fn wrapper") && l.contains("items: Vec<String>")),
        "wrapper() passes items to process() directly, should be owned. Generated: {}",
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
