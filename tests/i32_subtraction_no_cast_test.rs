//! TDD: Subtraction of two i32 fields should not insert any `as u32` casts.
//! Reproduces a pre-existing bug where the compiler incorrectly casts one operand
//! of an i32 subtraction to u32, causing type mismatches in downstream usage.

use std::process::Command;

fn compile_single(code: &str) -> String {
    let dir = tempfile::TempDir::new().unwrap();
    let src = dir.path().join("src_wj");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("inventory.wj"), code).unwrap();

    let wj = std::path::PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = Command::new(&wj)
        .arg("build")
        .arg(src.join("inventory.wj").to_str().unwrap())
        .arg("--no-cargo")
        .current_dir(dir.path())
        .output()
        .expect("wj build failed");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("wj build failed: {}", stderr);
    }

    let rs_path = dir.path().join("build").join("inventory.rs");
    std::fs::read_to_string(&rs_path).unwrap_or_else(|e| {
        panic!("Could not read generated Rust: {}", e);
    })
}

#[test]
fn test_i32_minus_i32_no_u32_cast() {
    let code = r#"
pub struct Item {
    pub max_stack: i32,
}

pub struct Stack {
    pub item: Item,
    pub quantity: i32,
}

pub fn compute_space(stack: Stack) -> i32 {
    let can_add = stack.item.max_stack - stack.quantity
    can_add
}
"#;

    let output = compile_single(code);

    assert!(
        !output.contains("as u32"),
        "i32 - i32 subtraction should NOT generate any `as u32` cast.\nGenerated:\n{}",
        output
    );
}

#[test]
fn test_i32_comparison_no_u32_cast() {
    let code = r#"
pub fn check(a: i32, b: i32) -> bool {
    let diff = a - b
    diff >= 0
}
"#;

    let output = compile_single(code);

    assert!(
        !output.contains("as u32"),
        "i32 comparison should NOT generate `as u32` cast.\nGenerated:\n{}",
        output
    );
}

#[test]
fn test_i32_compound_subtract_no_u32_cast() {
    let code = r#"
pub struct Counter {
    pub value: i32,
}

impl Counter {
    pub fn subtract(self, amount: i32) {
        self.value = self.value - amount
    }
}
"#;

    let output = compile_single(code);

    assert!(
        !output.contains("as u32"),
        "i32 compound subtraction should NOT generate `as u32` cast.\nGenerated:\n{}",
        output
    );
}
