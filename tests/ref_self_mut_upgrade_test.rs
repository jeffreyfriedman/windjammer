//! TDD Test: Methods that modify self fields should be &mut self even if user wrote &self
//!
//! When a user explicitly writes `&self` but the method modifies self fields,
//! the compiler should upgrade it to `&mut self` automatically.

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
#[cfg_attr(tarpaulin, ignore)]
fn test_ref_self_upgrade_to_mut() {
    // User wrote &self but method modifies field - should be upgraded to &mut self
    let code = r#"
pub struct Panel {
    visible: bool,
}

impl Panel {
    pub fn hide(&self) {
        self.visible = false
    }
}
"#;

    let (success, generated, err) = compile_and_get_generated(code);

    println!("Generated:\n{}", generated);
    if !success {
        println!("Compile error:\n{}", err);
    }

    assert!(success, "Compilation should succeed");

    // Should be upgraded to &mut self
    assert!(
        generated.contains("fn hide(&mut self)"),
        "hide should be &mut self (upgraded from &self). Generated:\n{}",
        generated
    );
}
