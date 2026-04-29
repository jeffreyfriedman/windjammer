use std::fs;
/// TDD Test: .clone() on borrowed strings should generate .to_string() when needed
///
/// Bug: When a string parameter is inferred as &str and .clone() is called on it,
/// and the result is passed to a function expecting String, the codegen generates
/// .clone() which returns &str, not String, causing E0308 type mismatch.
///
/// Fix: Detect when .clone() result needs to be String and generate .to_string() instead.
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_string_clone_generates_to_string() {
    let source = r#"
struct DialogTree {
    id: string,
}

impl DialogTree {
    pub fn new(id: string) -> DialogTree {
        DialogTree { id }
    }
}

pub fn create_dialog(id: string) -> DialogTree {
    DialogTree::new(id.clone())
}
"#;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let temp_file = temp_dir.path().join("test_string_clone.wj");
    let out_dir = temp_dir.path().join("out");
    fs::create_dir_all(&out_dir).unwrap();
    fs::write(&temp_file, source).unwrap();

    let wj_binary = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj_binary)
        .args([
            "build",
            temp_file.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj");

    let generated_path = out_dir.join("test_string_clone.rs");
    let generated = fs::read_to_string(&generated_path).unwrap();

    println!("Generated Rust:\n{}", generated);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("error[E0308]"),
        "Should not have type mismatch error. Stderr:\n{}",
        stderr
    );

    // Verify the generated code compiles with rustc
    let rustc_output = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg(&generated_path)
        .arg("-o")
        .arg(temp_dir.path().join("test.rlib"))
        .output()
        .expect("Failed to run rustc");
    let rustc_err = String::from_utf8_lossy(&rustc_output.stderr);
    assert!(
        rustc_output.status.success(),
        "Generated code should compile. Rustc stderr:\n{}",
        rustc_err
    );

    if generated.contains("id: &str") {
        assert!(
            generated.contains(".to_string()") || generated.contains(".to_owned()"),
            "Should convert &str to String with .to_string() or .to_owned(), not .clone()"
        );
    }
}

#[test]
fn test_owned_string_can_use_clone() {
    let source = r#"
struct DialogTree {
    id: string,
}

impl DialogTree {
    pub fn new(id: string) -> DialogTree {
        DialogTree { id }
    }
}

pub fn create_dialog(id: string, suffix: string) -> DialogTree {
    let full_id = format!("{}_{}", id, suffix)
    DialogTree::new(full_id.clone())
}
"#;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let temp_file = temp_dir.path().join("test_owned_string.wj");
    let out_dir = temp_dir.path().join("out");
    fs::create_dir_all(&out_dir).unwrap();
    fs::write(&temp_file, source).unwrap();

    let wj_binary = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj_binary)
        .args([
            "build",
            temp_file.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("error[E0308]"),
        "Should not have type mismatch error when cloning owned String. Stderr:\n{}",
        stderr
    );

    let generated_path = out_dir.join("test_owned_string.rs");
    let rustc_output = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg(&generated_path)
        .arg("-o")
        .arg(temp_dir.path().join("test.rlib"))
        .output()
        .expect("Failed to run rustc");
    let rustc_err = String::from_utf8_lossy(&rustc_output.stderr);
    assert!(
        rustc_output.status.success(),
        "Generated code should compile. Rustc stderr:\n{}",
        rustc_err
    );
}
