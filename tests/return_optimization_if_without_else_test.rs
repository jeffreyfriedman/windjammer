//! Return Optimization: Preserve explicit return in if-without-else
//!
//! Bug discovered via dogfooding windjammer-game:
//! The return optimization was converting `return value` to implicit `value`
//! inside if/if-let blocks without else branches, creating invalid Rust code.
//!
//! Example bad transformation:
//! ```wj
//! fn get_frame(self) -> usize {
//!     if let Some(frame) = self.frames.get(index) {
//!         return frame  // Explicit return needed!
//!     }
//!     0  // Default value
//! }
//! ```
//!
//! Generated (WRONG):
//! ```rust
//! fn get_frame(&self) -> usize {
//!     if let Some(frame) = self.frames.get(index) {
//!         frame  // ERROR: if without else can't have value!
//!     }
//!     0
//! }
//! ```
//!
//! Root Cause: Return optimization didn't check if we're inside an if-without-else.
//! In Rust, `if` without `else` must evaluate to `()`, so any value expression
//! (including implicit returns) is invalid.
//!
//! Fix: Preserve explicit `return` when inside if/if-let without corresponding else.

use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_wj_compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

fn compile_wj_source(source: &str) -> Result<String> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("wj_test_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    let source_file = temp_dir.join("test.wj");
    fs::write(&source_file, source)?;

    let output_dir = temp_dir.join("output");
    let wj = get_wj_compiler();
    let output = Command::new(wj)
        .arg("build")
        .arg(&source_file)
        .arg("--output")
        .arg(&output_dir)
        .arg("--target")
        .arg("rust")
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Compilation failed:\n{}", stderr);
    }

    let rust_code = fs::read_to_string(output_dir.join("test.rs"))?;
    fs::remove_dir_all(&temp_dir).ok();

    Ok(rust_code)
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_in_if_without_else_preserved() {
    // TDD RED: Return inside if-without-else must be preserved
    let source = r#"
pub fn check_condition(x: i32) -> bool {
    if x > 10 {
        return true
    }
    false
}
"#;

    let rust_code = compile_wj_source(source).expect("Compilation should succeed");

    // Verify: Generated Rust should have "return true", not just "true"
    assert!(
        rust_code.contains("return true"),
        "Expected 'return true' in generated code, got:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_in_if_let_without_else_preserved() {
    // TDD RED: Return inside if-let-without-else must be preserved
    let source = r#"
pub fn get_first(items: Vec<i32>) -> i32 {
    if let Some(first) = items.get(0) {
        return first
    }
    0
}
"#;

    let rust_code = compile_wj_source(source).expect("Compilation should succeed");

    // Verify: Generated Rust should have "return first", not just "first"
    assert!(
        rust_code.contains("return"),
        "Expected 'return' keyword in generated code, got:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_in_nested_if_without_else_preserved() {
    // TDD RED: Return inside nested if-without-else must be preserved
    let source = r#"
pub fn nested_check(a: bool, b: bool) -> i32 {
    if a {
        if b {
            return 1
        }
    }
    0
}
"#;

    let rust_code = compile_wj_source(source).expect("Compilation should succeed");

    // Verify: Generated Rust should have "return 1", not just "1"
    assert!(
        rust_code.contains("return 1"),
        "Expected 'return 1' in generated code, got:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_in_if_with_else_can_be_optimized() {
    // TDD GREEN: Return in if-else CAN be optimized (both branches have values)
    let source = r#"
pub fn with_else(x: i32) -> i32 {
    if x > 0 {
        return 1
    } else {
        return -1
    }
}
"#;

    let rust_code = compile_wj_source(source).expect("Compilation should succeed");

    // Verify: This could either have "return" or implicit returns
    // As long as it compiles to valid Rust, we're good
    // (The key test is that if-without-else preserves return)
    assert!(
        rust_code.contains("if") && rust_code.contains("else"),
        "Expected if-else structure in generated code"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_early_return_at_function_end() {
    // TDD GREEN: Return at the end of function CAN be optimized away
    let source = r#"
pub fn early_return(x: i32) -> i32 {
    let result = x * 2
    return result
}
"#;

    let rust_code = compile_wj_source(source).expect("Compilation should succeed");

    // Verify: This test just checks compilation succeeds
    // (Whether return is optimized away or not doesn't matter for correctness)
    assert!(rust_code.contains("fn early_return"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_dogfooding_animation_frame_getter() {
    // TDD RED: Real-world case from windjammer-game AnimationClip
    let source = r#"
pub struct Frame {
    pub index: usize,
}

pub struct AnimationClip {
    pub frames: Vec<Frame>,
}

impl AnimationClip {
    pub fn get_frame(self, index: usize) -> usize {
        if let Some(frame) = self.frames.get(index) {
            return frame.index
        }
        0
    }
}

fn main() {
    let clip = AnimationClip { frames: Vec::new() }
    println!("{}", clip.get_frame(0))
}
"#;

    let rust_code = compile_wj_source(source).expect("Compilation should succeed");

    // Verify: Generated Rust should have "return frame.index", not just "frame.index"
    assert!(
        rust_code.contains("return"),
        "Expected 'return' keyword in generated code, got:\n{}",
        rust_code
    );

    // CRITICAL: Generated Rust must be valid (no "expected (), found usize" error)
    // This is tested implicitly by compile_wj_source succeeding
}
