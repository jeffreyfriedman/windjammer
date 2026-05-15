use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_match_arm_block_comma_optional() {
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj");

    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_match_block");

    fs::create_dir_all(&test_dir).unwrap();

    // Test that commas are optional after block expressions in match arms
    let test_content = r#"
fn check(x: i32) -> bool {
    match x {
        0 => { false }
        1 => { true }
        _ => false
    }
}

fn main() {
    let result = check(1);
}
"#;

    let test_file = test_dir.join("test.wj");
    fs::write(&test_file, test_content).unwrap();

    let output = Command::new(&wj_binary)
        .current_dir(&test_dir)
        .arg("build")
        .arg(&test_file)
        .arg("--no-cargo")
        .output()
        .expect("Failed to execute wj build");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("STDOUT:\n{}", stdout);
    println!("STDERR:\n{}", stderr);

    // The test should parse and transpile successfully
    // We use --no-run-cargo to avoid codegen issues unrelated to parsing
    assert!(
        output.status.success() || stdout.contains("Success! Transpilation complete!"),
        "Expected transpilation to succeed, but it failed.\nSTDOUT: {}\nSTDERR: {}",
        stdout,
        stderr
    );

    // Check that the generated Rust code was created
    let rust_file = test_dir.join("build").join("test.rs");
    assert!(
        rust_file.exists(),
        "Expected generated Rust file to exist at {:?}",
        rust_file
    );

    // Clean up
    let _ = fs::remove_dir_all(&test_dir);
}
