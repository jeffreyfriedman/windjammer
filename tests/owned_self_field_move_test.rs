/// TDD Tests: Ownership inference for non-Copy field moves (v0.41.0)
///
/// When a method returns `self.field` where the field is a non-Copy type (e.g. String),
/// the compiler must infer owned `self` (not `&self`), because you can't move a field
/// out of a borrowed reference.
///
/// This was a known issue from v0.40.0.
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_and_get_rust(source: &str) -> String {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    let generated_file = temp_dir.path().join("build").join("test.rs");
    fs::read_to_string(&generated_file).unwrap_or_else(|_| {
        panic!(
            "Failed to read generated file. Compiler stderr:\n{}",
            String::from_utf8_lossy(&wj_output.stderr)
        )
    })
}

// ==========================================
// Return self.field (non-Copy) → owned self
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_self_string_field_infers_owned_self() {
    let generated = compile_and_get_rust(
        r#"
struct Text {
    content: string
}

impl Text {
    fn get_content(self) -> string {
        self.content
    }
}
"#,
    );

    // The method returns self.content (a String, non-Copy).
    // Moving a field out of self requires owned self, not &self.
    assert!(
        generated.contains("fn get_content(self) -> String"),
        "Expected owned `self` when returning non-Copy field. Got:\n{}",
        generated
    );
    assert!(
        !generated.contains("fn get_content(&self)"),
        "Should NOT use &self when returning non-Copy field. Got:\n{}",
        generated
    );
}

// ==========================================
// Return self.field (Copy type) → &self OK
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_self_copy_field_allows_borrowed_self() {
    let generated = compile_and_get_rust(
        r#"
struct Counter {
    value: int
}

impl Counter {
    fn get_value(self) -> int {
        self.value
    }
}
"#,
    );

    // int is Copy, so reading self.value works fine with &self
    // The compiler can (and should) use &self here for efficiency
    assert!(
        generated.contains("fn get_value(&self) -> i64"),
        "Expected &self when returning Copy field. Got:\n{}",
        generated
    );
}

// ==========================================
// Return self.field (Vec, non-Copy) → owned self
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_self_vec_field_infers_owned_self() {
    let generated = compile_and_get_rust(
        r#"
struct Container {
    items: Vec<int>
}

impl Container {
    fn take_items(self) -> Vec<int> {
        self.items
    }
}
"#,
    );

    assert!(
        generated.contains("fn take_items(self) -> Vec<i64>"),
        "Expected owned `self` when returning non-Copy Vec field. Got:\n{}",
        generated
    );
}
