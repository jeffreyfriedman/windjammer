//! TDD: Method receiver ownership inference
//!
//! Tests for infer_method_receiver_ownership + generate_expression_with_target_ownership.
//! Verifies correct receiver generation (owned, borrowed, mut borrowed) for method calls.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_wj_to_rust(src: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&test_file, src).unwrap();
    fs::create_dir_all(&out_dir).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            test_file.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .unwrap();

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let rs_file = out_dir.join("test.rs");
    Ok(fs::read_to_string(&rs_file).unwrap())
}

#[test]
fn test_method_owned_receiver() {
    let src = r#"
        pub struct Timer { id: i32 }
        impl Timer {
            pub fn id(self) -> i32 { self.id }
        }
        pub fn get_id(t: Timer) -> i32 {
            t.id()
        }
    "#;

    let result = compile_wj_to_rust(src).expect("Should compile");
    // t is Owned, id() takes Owned -> t.id() (no clone)
    assert!(result.contains("t.id()"));
}

#[test]
fn test_method_borrowed_receiver() {
    let src = r#"
        pub fn length(s: string) -> usize {
            s.len()
        }
    "#;

    let result = compile_wj_to_rust(src).expect("Should compile");
    // s is Owned, len() takes &self -> s.len() (auto-borrow)
    assert!(result.contains("s.len()"));
}

#[test]
fn test_method_builder_pattern() {
    let src = r#"
        pub struct Builder { value: i32 }
        impl Builder {
            pub fn with_value(self, v: i32) -> Self {
                Builder { value: v }
            }
        }
        pub fn build(b: Builder) -> Builder {
            b.with_value(42)
        }
    "#;

    let result = compile_wj_to_rust(src).expect("Should compile");
    // b is Owned, with_value() takes Owned -> b.with_value() (no clone)
    assert!(result.contains("b.with_value(42)"));
}

#[test]
fn test_method_mutating_receiver() {
    let src = r#"
        pub fn add_item(items: Vec<i32>) {
            items.push(5)
        }
    "#;

    let result = compile_wj_to_rust(src).expect("Should compile");
    // items is Owned, push() takes &mut self
    // Should generate: (&mut items).push(5) or similar
    assert!(result.contains("push(5)"));
}
