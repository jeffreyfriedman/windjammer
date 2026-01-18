use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_string_param_stays_owned() {
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj");

    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_string_ownership");
    
    fs::create_dir_all(&test_dir).unwrap();

    // Test that string parameters stay as owned String, not &String
    let test_content = r#"
fn print_message(message: string) {
    println!("{}", message);
}

fn main() {
    let msg = "Hello".to_string();
    print_message(msg);
}
"#;

    let test_file = test_dir.join("string_ownership.wj");
    fs::write(&test_file, test_content).unwrap();

    let output = Command::new(&wj_binary)
        .current_dir(&test_dir)
        .arg("build")
        .arg(&test_file)
        .output()
        .expect("Failed to execute wj build");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("STDOUT:\n{}", stdout);
    println!("STDERR:\n{}", stderr);

    let rust_file = test_dir.join("build").join("string_ownership.rs");
    let rust_code = fs::read_to_string(&rust_file).unwrap();
    println!("Generated Rust:\n{}", rust_code);

    // The parameter should be `message: String`, NOT `message: &String`
    assert!(
        rust_code.contains("fn print_message(message: String)"),
        "Expected owned String parameter, not &String.\nGenerated code:\n{}",
        rust_code
    );

    // Verify it compiles
    let compile_output = Command::new("rustc")
        .current_dir(test_dir.join("build"))
        .arg("--crate-type")
        .arg("bin")
        .arg("string_ownership.rs")
        .output()
        .expect("Failed to run rustc");

    let compile_stderr = String::from_utf8_lossy(&compile_output.stderr);
    assert!(
        compile_output.status.success(),
        "Expected generated code to compile.\nRustc errors:\n{}",
        compile_stderr
    );

    // Clean up
    let _ = fs::remove_dir_all(&test_dir);
}

