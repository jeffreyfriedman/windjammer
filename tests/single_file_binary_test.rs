use std::path::PathBuf;
/// TDD Test: Single-file binary compilation
///
/// This test ensures single-file Windjammer programs compile to executables
/// with correct Cargo.toml [[bin]] sections.
use std::process::Command;

fn get_wj_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

#[test]
fn test_single_file_creates_bin_target() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_dir = temp_dir.path();

    let wj_code = r#"
fn main() {
    println!("Hello, world!")
}
"#;

    // Write Windjammer source file
    let input_file = temp_dir.path().join("hello.wj");
    std::fs::write(&input_file, wj_code).expect("Failed to write input file");

    println!("🔨 Compiling hello.wj...");

    // Compile with --no-cargo to see what files are generated
    let output = Command::new(get_wj_binary())
        .arg("build")
        .arg(&input_file)
        .arg("--output")
        .arg(output_dir)
        .arg("--no-cargo")
        .output()
        .expect("Failed to execute wj compiler");

    if !output.status.success() {
        eprintln!("❌ Compilation failed!");
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("hello.wj compilation failed");
    }

    println!("✅ Compilation succeeded");

    // Check that hello.rs was created
    let hello_rs = output_dir.join("hello.rs");
    assert!(
        hello_rs.exists(),
        "Expected hello.rs to exist at {:?}, but it doesn't!\nFiles in output_dir: {:?}",
        hello_rs,
        std::fs::read_dir(output_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name())
            .collect::<Vec<_>>()
    );

    // Check that Cargo.toml was created
    let cargo_toml = output_dir.join("Cargo.toml");
    assert!(
        cargo_toml.exists(),
        "Expected Cargo.toml to exist at {:?}",
        cargo_toml
    );

    // Check that Cargo.toml has [[bin]] section
    let cargo_toml_content =
        std::fs::read_to_string(&cargo_toml).expect("Failed to read Cargo.toml");

    assert!(
        cargo_toml_content.contains("[[bin]]"),
        "Cargo.toml should contain [[bin]] section, got:\n{}",
        cargo_toml_content
    );

    assert!(
        cargo_toml_content.contains("name = \"hello\""),
        "Cargo.toml should contain bin name, got:\n{}",
        cargo_toml_content
    );

    assert!(
        cargo_toml_content.contains("path = \"hello.rs\""),
        "Cargo.toml should contain bin path, got:\n{}",
        cargo_toml_content
    );

    println!("✅ Cargo.toml has correct [[bin]] section");
}
