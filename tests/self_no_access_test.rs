//! TDD Test: Methods that don't access self at all should still be &self
//!
//! When a method doesn't access self.field at all, it should still be &self
//! since there's no reason to consume it.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_and_get_generated(code: &str) -> (bool, String, String) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, code).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return (false, String::new(), stderr);
    }

    let generated_path = out_dir.join("test.rs");
    let generated = fs::read_to_string(&generated_path)
        .unwrap_or_else(|_| "Failed to read generated file".to_string());

    (true, generated, String::new())
}

#[test]
fn test_method_no_self_access_should_borrow() {
    // Method that doesn't access self at all should be &self
    let code = r#"
pub struct Helper {
    data: string,
}

impl Helper {
    pub fn format_label(self, label: string) -> string {
        format!("Label: {}", label)
    }
}
"#;

    let (success, generated, err) = compile_and_get_generated(code);

    println!("Generated:\n{}", generated);
    if !success {
        println!("Compile error:\n{}", err);
    }

    assert!(success, "Compilation should succeed");

    // Method should have &self, not self - even though it doesn't use self
    assert!(
        generated.contains("fn format_label(&self"),
        "format_label should be &self. Generated:\n{}",
        generated
    );
}

