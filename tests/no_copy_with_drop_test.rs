#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

use std::process::Command;

fn compile_wj_to_rs(source: &str) -> (bool, String, String) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let input = dir.path().join("test.wj");
    std::fs::write(&input, source).expect("write test.wj");
    let output = dir.path().join("output");
    std::fs::create_dir_all(&output).expect("create output dir");

    let result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", input.to_str().unwrap(), "--no-cargo", "-o"])
        .arg(output.to_str().unwrap())
        .output()
        .expect("run wj");

    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    let stderr = String::from_utf8_lossy(&result.stderr).to_string();
    let combined = format!("{}\n{}", stdout, stderr);

    let generated_path = output.join("test.rs");
    let generated = if generated_path.exists() {
        std::fs::read_to_string(&generated_path).unwrap_or_default()
    } else {
        String::new()
    };

    (result.status.success(), generated, combined)
}

/// Types with Drop impls must NOT auto-derive Copy (Rust E0184).
/// Even if all fields are Copy, having a destructor makes Copy invalid.
#[test]
fn test_no_copy_derive_when_drop_impl_exists() {
    let source = r#"
pub struct Handle {
    pub id: u64,
}

impl Handle {
    pub fn new(id: u64) -> Handle {
        Handle { id }
    }
}

impl Drop for Handle {
    fn drop(self) {
        if self.id != 0 {
            eprintln!("Dropping handle {}", self.id)
        }
    }
}
"#;
    let (success, generated, output) = compile_wj_to_rs(source);
    assert!(success, "WJ compilation should succeed: {}", output);

    // Should NOT derive Copy since there's a Drop impl
    assert!(
        !generated.contains("Copy"),
        "Struct with Drop impl should NOT derive Copy.\nGenerated:\n{}",
        generated
    );

    // Should still derive Debug, Clone
    assert!(
        generated.contains("Debug"),
        "Struct should still derive Debug.\nGenerated:\n{}",
        generated
    );
    assert!(
        generated.contains("Clone"),
        "Struct should still derive Clone.\nGenerated:\n{}",
        generated
    );
}
