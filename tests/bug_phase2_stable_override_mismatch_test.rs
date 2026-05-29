#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
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

/// Phase 2 optimizes `fn add_class(self, class: string)` to take `&str` because
/// the param is only forwarded to an extern fn. The generated function definition
/// should use `&str`, and callers that pass string literals should NOT get spurious `&`.
///
/// Bug: callers generated `&"literal"` (&&str) instead of `"literal"` (&str).
/// Root cause: `.to_string()` was stripped from `"literal".to_string()`, then
/// `should_add_ref` saw the original MethodCall expression and added `&`.
#[test]
fn test_phase2_caller_no_double_ref_on_string_literal() {
    let source = r#"
extern fn apply_class(handle: i32, class: string)

pub struct Node {
    handle: i32,
}

impl Node {
    pub fn new() -> Node {
        Node { handle: 1 }
    }

    pub fn add_class(self, class: string) -> Node {
        apply_class(self.handle, class)
        self
    }
}

pub fn build_node() -> Node {
    let node = Node::new()
        .add_class("my-class".to_string())
    node
}
"#;

    let (ok, generated, diagnostics) = compile_wj_to_rs(source);
    assert!(ok, "compilation failed: {}", diagnostics);

    // The generated code for the caller should NOT have &"my-class" (double ref)
    // It should be just "my-class" (string literal is already &str)
    assert!(
        !generated.contains(r#"&"my-class""#),
        "Generated code has &&str (double ref on string literal):\n{}",
        generated
    );
}

/// When Phase 2 optimizes a method to take &str, callers passing variables
/// should get `&variable` (which auto-derefs String to &str), NOT double refs.
#[test]
fn test_phase2_caller_variable_arg_correct_ref() {
    let source = r#"
extern fn apply_class(handle: i32, class: string)

pub struct Node {
    handle: i32,
}

impl Node {
    pub fn new() -> Node {
        Node { handle: 1 }
    }

    pub fn add_class(self, class: string) -> Node {
        apply_class(self.handle, class)
        self
    }
}

pub fn build_node() -> Node {
    let class_name = "dynamic-class"
    let node = Node::new()
        .add_class(class_name.to_string())
    node
}
"#;

    let (ok, generated, diagnostics) = compile_wj_to_rs(source);
    assert!(ok, "compilation failed: {}", diagnostics);

    // class_name is already &str, should not get .to_string() or spurious & 
    // The generated code should NOT have &&class_name or &class_name.to_string()
    // Since class_name is &str and callee wants &str, just pass class_name directly
    let has_double_ref = generated.contains("&&class_name");
    assert!(
        !has_double_ref,
        "Generated code has double-ref on variable:\n{}",
        generated
    );
}
