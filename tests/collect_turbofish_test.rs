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

/// When `let chars: Vec<char> = text.chars().collect()`, the compiler should
/// generate `collect::<Vec<char>>()`, not `collect::<String>()`.
#[test]
fn test_collect_uses_let_binding_type() {
    let source = r#"
pub struct TextInput {
    pub text: string,
}

impl TextInput {
    pub fn get_chars(self) -> Vec<char> {
        let chars: Vec<char> = self.text.chars().collect()
        chars
    }
}
"#;
    let (success, generated, output) = compile_wj_to_rs(source);
    assert!(success, "WJ compilation should succeed: {}", output);

    assert!(
        generated.contains("collect::<Vec<char>>()"),
        "collect() should use Vec<char> turbofish from let binding, not return type.\nGenerated:\n{}",
        generated
    );
    assert!(
        !generated.contains("collect::<String>()"),
        "collect() should NOT use String (function return type) as turbofish.\nGenerated:\n{}",
        generated
    );
}

/// When let binding has Vec<char> type but collect is not the last expression,
/// it should still use Vec<char>.
#[test]
fn test_collect_with_mut_let_binding_type() {
    let source = r#"
pub fn split_chars(text: string) -> i32 {
    let mut chars: Vec<char> = text.chars().collect()
    chars.len() as i32
}
"#;
    let (success, generated, output) = compile_wj_to_rs(source);
    assert!(success, "WJ compilation should succeed: {}", output);

    assert!(
        generated.contains("collect::<Vec<char>>()"),
        "mut let binding collect() should use Vec<char> turbofish.\nGenerated:\n{}",
        generated
    );
}
