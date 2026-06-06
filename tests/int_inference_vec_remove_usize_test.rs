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

/// TDD Test: Vec::remove(0) should infer usize, not i32
///
/// Bug: `self.entries.remove(0)` generates `0_i32` instead of `0_usize`.
/// The int inference engine needs to detect that `remove()` on a Vec field
/// requires a `usize` index parameter.
use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_vec_remove_on_self_field_infers_usize() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");

    fs::write(
        &test_file,
        r#"
struct Entry {
    message: String,
}

struct Console {
    entries: Vec<Entry>,
    max_entries: i32,
}

impl Console {
    fn trim(self) {
        if self.entries.len() > 100 {
            self.entries.remove(0)
        }
    }
}

pub fn main() {
    let mut console = Console { entries: Vec::new(), max_entries: 500 }
    console.trim()
}
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("wj").unwrap();
    let output = cmd
        .arg("build")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Compilation should succeed!\n\nStderr:\n{}",
        stderr
    );

    let generated = fs::read_to_string(temp_dir.path().join("build/test.rs"))
        .expect("Generated Rust file should exist at build/test.rs");

    assert!(
        generated.contains("remove(0_usize)"),
        "Should generate remove(0_usize) for Vec::remove()! Generated:\n{}",
        generated
    );
}

#[test]
fn test_vec_remove_on_nested_self_field_infers_usize() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");

    fs::write(
        &test_file,
        r#"
struct Action {
    name: String,
}

struct UndoManager {
    undo_stack: Vec<Action>,
}

impl UndoManager {
    fn limit_history(self) {
        if self.undo_stack.len() > 50 {
            self.undo_stack.remove(0)
        }
    }
}

pub fn main() {
    let mut mgr = UndoManager { undo_stack: Vec::new() }
    mgr.limit_history()
}
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("wj").unwrap();
    let output = cmd
        .arg("build")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Compilation should succeed!\n\nStderr:\n{}",
        stderr
    );

    let generated = fs::read_to_string(temp_dir.path().join("build/test.rs"))
        .expect("Generated Rust file should exist at build/test.rs");

    assert!(
        generated.contains("remove(0_usize)"),
        "Should generate remove(0_usize) for Vec::remove()! Generated:\n{}",
        generated
    );
}
