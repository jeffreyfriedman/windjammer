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

/// When a method calls self.field.method() where the field method takes &mut self,
/// the outer method should be &mut self (not &self)
#[test]
fn test_method_calling_field_mutating_method_gets_mut_self() {
    let source = r#"
pub struct Inner {
    pub value: i32,
}

impl Inner {
    pub fn increment(self) {
        self.value = self.value + 1
    }
}

pub struct Outer {
    pub inner: Inner,
}

impl Outer {
    pub fn do_work(self) {
        self.inner.increment()
    }
}
"#;
    let (success, generated, output) = compile_wj_to_rs(source);
    assert!(success, "WJ compilation should succeed: {}", output);

    assert!(
        generated.contains("fn do_work(&mut self)"),
        "Method calling self.field.mutating_method() should be &mut self.\nGenerated:\n{}",
        generated
    );
}
