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

/// When a Windjammer function takes `string` and the analyzer infers `&`,
/// the codegen should generate `&str` (not `&String`), because:
/// - `&str` accepts both `&String` and `&str` literals
/// - `&String` only accepts `&String`, rejecting `&str` literals
#[test]
fn test_borrowed_string_param_generates_str_ref() {
    let source = r#"
pub fn greet(name: string) {
    eprintln!("Hello, {}", name)
}
"#;
    let (success, generated, output) = compile_wj_to_rs(source);
    assert!(success, "WJ compilation should succeed: {}", output);

    assert!(
        generated.contains("fn greet(name: &str)"),
        "Borrowed string param should be &str, not &String.\nGenerated:\n{}",
        generated
    );
}

/// When a function passes a string param to another function, it should be &str
#[test]
fn test_string_param_passed_to_fn_call() {
    let source = r#"
fn inner(s: string) -> i32 {
    42
}

pub fn outer(name: string) -> i32 {
    inner(name)
}
"#;
    let (success, generated, output) = compile_wj_to_rs(source);
    assert!(success, "WJ compilation should succeed: {}", output);

    assert!(
        !generated.contains("name: &String"),
        "Borrowed string param should be &str, not &String.\nGenerated:\n{}",
        generated
    );
}

/// When a function passes a string to an extern fn, the param should still be &str
/// because the codegen wraps extern fn string args in string_to_ffi(.to_string())
#[test]
fn test_extern_fn_wrapper_uses_str_ref() {
    let source = r#"
extern fn gpu_env_var_is_set(name: &str) -> u32

pub fn env_var_is_set(name: string) -> bool {
    gpu_env_var_is_set(name) != 0
}
"#;
    let (success, generated, output) = compile_wj_to_rs(source);
    assert!(success, "WJ compilation should succeed: {}", output);

    assert!(
        generated.contains("fn env_var_is_set(name: &str)"),
        "Wrapper around extern fn should use &str, not &String.\nGenerated:\n{}",
        generated
    );
}

/// When a function passes a string to a path-qualified function, &str should work
#[test]
fn test_path_qualified_call_uses_str_ref() {
    let source = r#"
pub fn log_message(msg: string) {
    eprintln!("{}", msg)
}

pub fn warn(msg: string) {
    log_message(msg)
}
"#;
    let (success, generated, output) = compile_wj_to_rs(source);
    assert!(success, "WJ compilation should succeed: {}", output);

    assert!(
        generated.contains("fn warn(msg: &str)"),
        "Function passing string to another fn should use &str.\nGenerated:\n{}",
        generated
    );
}

/// String params in methods should also generate &str when borrowed
#[test]
fn test_method_borrowed_string_param() {
    let source = r#"
pub struct Logger {
    pub prefix: string,
}

impl Logger {
    pub fn log(self, message: string) {
        eprintln!("[{}] {}", self.prefix, message)
    }
}
"#;
    let (success, generated, output) = compile_wj_to_rs(source);
    assert!(success, "WJ compilation should succeed: {}", output);

    assert!(
        generated.contains("fn log(&self, message: &str)"),
        "Borrowed string param in method should be &str.\nGenerated:\n{}",
        generated
    );
}
