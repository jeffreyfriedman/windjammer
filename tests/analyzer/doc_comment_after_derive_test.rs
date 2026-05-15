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

#[path = "../common/test_utils.rs"]
mod test_utils;

use std::fs;
/// TDD Test: DocComment after @derive should parse
///
/// Bug: Parser rejected `@derive(Clone)\n/// doc\npub struct S` with
/// "Unexpected token: DocComment(...)". Doc comments between @derive and struct
/// should be accepted.
///
/// Source: prefab_system.wj, crafting.wj in windjammer-game-core
use std::process::Command;

#[test]
fn test_doc_comment_after_derive_parses() {
    let source = r#"
@derive(Clone)
/// A reusable entity template
pub struct Prefab {
    pub id: i32,
    pub name: string,
}

impl Prefab {
    pub fn new(id: i32, name: string) -> Prefab {
        Prefab { id: id, name: name }
    }
}
"#;

    let _tmp = tempfile::tempdir().unwrap();

    let temp_dir = _tmp
        .path()
        .join(format!("wj_doc_derive_test_{}", std::process::id()));

    fs::create_dir_all(&temp_dir).unwrap();
    let temp_file = temp_dir.join("test.wj");
    let output_dir = temp_dir.join("out");
    fs::create_dir_all(&output_dir).unwrap();
    fs::write(&temp_file, source).unwrap();

    let output = Command::new(test_utils::wj_binary())
        .args([
            "build",
            "--output",
            output_dir.to_str().unwrap(),
            "--target",
            "rust",
            temp_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run wj");

    assert!(
        output.status.success(),
        "DocComment after @derive should parse. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_doc_comment_before_derive_parses() {
    // Workaround: doc comment before @derive parses correctly
    let source = r#"
/// A reusable entity template
@derive(Clone)
pub struct Prefab {
    pub id: i32,
    pub name: string,
}

impl Prefab {
    pub fn new(id: i32, name: string) -> Prefab {
        Prefab { id: id, name: name }
    }
}
"#;

    let _tmp2 = tempfile::tempdir().unwrap();

    let temp_dir = _tmp2
        .path()
        .join(format!("wj_doc_derive_test2_{}", std::process::id()));

    fs::create_dir_all(&temp_dir).unwrap();
    let temp_file = temp_dir.join("test.wj");
    let output_dir = temp_dir.join("out");
    fs::create_dir_all(&output_dir).unwrap();
    fs::write(&temp_file, source).unwrap();

    let output = Command::new(test_utils::wj_binary())
        .args([
            "build",
            "--output",
            output_dir.to_str().unwrap(),
            "--target",
            "rust",
            temp_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run wj");

    assert!(
        output.status.success(),
        "DocComment before @derive should parse. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
