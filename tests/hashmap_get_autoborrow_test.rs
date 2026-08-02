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

/// app: `query.get(key)` on `std::collections::HashMap` must emit `&key`, not `key` or `&&key`.
#[test]
fn test_std_collections_hashmap_get_borrows_owned_key() {
    let source = r#"
use std::collections::HashMap

fn query_get_owned(query: std::collections::HashMap<string, string>, key: string) -> Option<string> {
    match query.get(key) {
        Some(value) => Some(value.to_string()),
        None => None,
    }
}
"#;

    let (success, generated, output) = compile_wj_to_rs(source);
    assert!(success, "HashMap get with owned key should compile: {output}");
    assert!(
        generated.contains(".get(&key)") || generated.contains(".get(key)"),
        "expected `.get(&key)` (owned String) or `.get(key)` (&str formal), got:\n{generated}"
    );
    assert!(
        !generated.contains(".get(&&key)"),
        "must not double-borrow key:\n{generated}"
    );
}

/// app route helpers: owned `string` formals pass by value at call sites.
#[test]
fn test_module_helper_owned_string_call_no_spurious_borrow() {
    let source = r#"
fn extract_path(path: string) -> string {
    path
}

fn caller(p: string) -> string {
    extract_path(p)
}
"#;

    let (success, generated, output) = compile_wj_to_rs(source);
    assert!(success, "{output}");
    assert!(
        !generated.contains("extract_path(&p)"),
        "spurious borrow on owned string formal:\n{generated}"
    );
}
