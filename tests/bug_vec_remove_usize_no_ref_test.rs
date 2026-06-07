#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

/// TDD Test: Vec.remove(index) should NOT add & to the index argument
///
/// Problem: Vec::remove takes an owned `usize` index, but the compiler
/// incorrectly adds `&` to the argument (treating it like HashMap::remove
/// which takes &K).
///
/// Example from text_input.wj:
///   let pos: usize = (self.cursor_position - 1) as usize
///   codepoints.remove(pos)
///
/// Generated (wrong): codepoints.remove(&pos);
/// Expected:          codepoints.remove(pos);
use std::process::Command;

#[test]
fn test_vec_remove_usize_no_ref() {
    let source = r#"
fn main() {
    let mut items = vec![10, 20, 30, 40, 50]
    let pos: usize = 2
    items.remove(pos)
    println("{}", items.len())
}
"#;

    let _tmp = tempfile::tempdir().unwrap();
    let temp_dir = _tmp.path();
    let test_id = format!(
        "wj_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let output_dir = temp_dir.join(&test_id);

    let source_file = temp_dir.join("test_vec_remove.wj");
    std::fs::write(&source_file, source).unwrap();

    let wj = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj)
        .arg("build")
        .arg(source_file.to_str().unwrap())
        .arg("--output")
        .arg(output_dir.to_str().unwrap())
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj");

    assert!(
        output.status.success(),
        "wj build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let rs_file = output_dir.join("test_vec_remove.rs");
    assert!(rs_file.exists(), "Generated .rs file not found");

    let generated = std::fs::read_to_string(&rs_file).unwrap();

    // Vec::remove takes owned usize, NOT &usize
    assert!(
        !generated.contains("items.remove(&pos)"),
        "BUG: Vec.remove adds & to usize index!\nGenerated:\n{}",
        generated
    );

    // Should be plain items.remove(pos)
    assert!(
        generated.contains("items.remove(pos)"),
        "Expected items.remove(pos) without &.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_vec_remove_with_expression_no_ref() {
    let source = r#"
struct TextInput {
    text: String,
    cursor_position: i32,
}

impl TextInput {
    pub fn delete_char(self) {
        if self.cursor_position > 0 {
            let pos: usize = (self.cursor_position - 1) as usize
            let mut codepoints = self.text.chars().collect::<Vec<char>>()
            if pos < codepoints.len() {
                codepoints.remove(pos)
                self.cursor_position = self.cursor_position - 1
            }
        }
    }
}

fn main() {
    let mut input = TextInput { text: String::from("hello"), cursor_position: 3 }
    input.delete_char()
    println("{}", input.cursor_position)
}
"#;

    let _tmp = tempfile::tempdir().unwrap();
    let temp_dir = _tmp.path();
    let test_id = format!(
        "wj_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let output_dir = temp_dir.join(&test_id);

    let source_file = temp_dir.join("test_vec_remove_expr.wj");
    std::fs::write(&source_file, source).unwrap();

    let wj = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj)
        .arg("build")
        .arg(source_file.to_str().unwrap())
        .arg("--output")
        .arg(output_dir.to_str().unwrap())
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj");

    assert!(
        output.status.success(),
        "wj build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let rs_file = output_dir.join("test_vec_remove_expr.rs");
    assert!(rs_file.exists(), "Generated .rs file not found");

    let generated = std::fs::read_to_string(&rs_file).unwrap();

    // Vec::remove takes owned usize, NOT &usize
    assert!(
        !generated.contains("codepoints.remove(&pos)"),
        "BUG: Vec.remove adds & to usize index!\nGenerated:\n{}",
        generated
    );

    assert!(
        generated.contains("codepoints.remove(pos)"),
        "Expected codepoints.remove(pos) without &.\nGenerated:\n{}",
        generated
    );
}
