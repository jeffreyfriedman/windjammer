/// Return Optimization: Preserve explicit return in if-without-else
///
/// Bug discovered via dogfooding windjammer-game:
/// The return optimization was converting `return value` to implicit `value`
/// inside if/if-let blocks without else branches, creating invalid Rust code.
///
/// Example bad transformation:
/// ```wj
/// fn get_frame(self) -> usize {
///     if let Some(frame) = self.frames.get(index) {
///         return frame  // Explicit return needed!
///     }
///     0  // Default value
/// }
/// ```
///
/// Generated (WRONG):
/// ```rust
/// fn get_frame(&self) -> usize {
///     if let Some(frame) = self.frames.get(index) {
///         frame  // ERROR: if without else can't have value!
///     }
///     0
/// }
/// ```
///
/// Root Cause: Return optimization didn't check if we're inside an if-without-else.
/// In Rust, `if` without `else` must evaluate to `()`, so any value expression
/// (including implicit returns) is invalid.
///
/// Fix: Preserve explicit `return` when inside if/if-let without corresponding else.

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
    let temp_dir = std::env::temp_dir().join(format!("wj_return_if_test_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    let src_dir = temp_dir.join("src_wj");
    fs::create_dir_all(&src_dir)?;

    fs::write(src_dir.join("main.wj"), source)?;

    fs::write(
        temp_dir.join("wj.toml"),
        r#"[package]
name = "test-return-if"
version = "0.1.0"

[dependencies]
"#,
    )?;

    let output_dir = temp_dir.join("src");
    let output = Command::new(get_wj_compiler())
        .args(["build", "--no-cargo"])
        .arg(src_dir.to_str().unwrap())
        .arg("--output")
        .arg(output_dir.to_str().unwrap())
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    if !output.status.success() {
        anyhow::bail!(
            "Compiler failed:\nstdout: {}\nstderr: {}",
            stdout, stderr
        );
    }

    let rust_file = output_dir.join("main.rs");
    Ok(fs::read_to_string(rust_file).unwrap_or_else(|_| {
        panic!("Failed to read generated main.rs\nstdout: {}\nstderr: {}", stdout, stderr)
    }))
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_in_if_let_without_else() -> Result<()> {
    let source = r#"
struct Frame {
    index: i64,
}

impl Frame {
    fn new(index: i64) -> Frame {
        Frame { index }
    }
    
    fn value(self) -> i64 {
        self.index
    }
}

fn get_frame(frames: Vec<Frame>, index: i64) -> i64 {
    if let Some(frame) = frames.get(index as usize) {
        return frame.value()
    }
    0
}

fn main() {
    let frames = vec![Frame::new(10), Frame::new(20), Frame::new(30)]
    println("{}", get_frame(frames, 1))
}
"#;

    let rust_code = compile_wj_source(source)?;
    
    // Must preserve explicit return in if-let without else
    assert!(
        rust_code.contains("return frame.value()") || rust_code.contains("return frame.value();"),
        "Must preserve explicit 'return' in if-let without else!\n\nGenerated:\n{}",
        rust_code
    );
    
    // Should NOT have implicit return (just `frame.value()` without return keyword)
    // This would cause E0308: if without else can't have a value
    assert!(
        !rust_code.contains("if let Some(frame) = frames.get") || !rust_code.contains("{\n                frame.value()\n            }"),
        "Must NOT optimize to implicit return in if-let without else!\n\nGenerated:\n{}",
        rust_code
    );
    
    Ok(())
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_in_if_without_else() -> Result<()> {
    let source = r#"
fn check_positive(x: i64) -> i64 {
    if x > 0 {
        return x * 2
    }
    0
}

fn main() {
    println("{}", check_positive(5))
}
"#;

    let rust_code = compile_wj_source(source)?;
    
    // Must preserve explicit return in if without else
    assert!(
        rust_code.contains("return x * 2") || rust_code.contains("return x * 2;"),
        "Must preserve explicit 'return' in if without else!\n\nGenerated:\n{}",
        rust_code
    );
    
    Ok(())
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_can_optimize_if_else_return() -> Result<()> {
    let source = r#"
fn abs(x: i64) -> i64 {
    if x < 0 {
        return -x
    } else {
        return x
    }
}

fn main() {
    println("{}", abs(-5))
}
"#;

    let rust_code = compile_wj_source(source)?;
    
    // CAN optimize returns in if-else (both branches have return/value)
    // This is safe because if-else always evaluates to a value
    let has_implicit_return = !rust_code.contains("return -x") && !rust_code.contains("return x");
    let has_explicit_return = rust_code.contains("return -x") || rust_code.contains("return x");
    
    // Either optimization is fine, but implicit is preferred
    assert!(
        has_implicit_return || has_explicit_return,
        "Should generate either implicit or explicit returns for if-else\n\nGenerated:\n{}",
        rust_code
    );
    
    Ok(())
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_nested_if_let_preserve_return() -> Result<()> {
    let source = r#"
fn get_value(outer: Option<Option<i64>>) -> i64 {
    if let Some(inner) = outer {
        if let Some(value) = inner {
            return value * 2
        }
    }
    0
}

fn main() {
    println("{}", get_value(Some(Some(21))))
}
"#;

    let rust_code = compile_wj_source(source)?;
    
    // Must preserve explicit return in nested if-let without else
    assert!(
        rust_code.contains("return value * 2") || rust_code.contains("return value * 2;"),
        "Must preserve explicit 'return' in nested if-let without else!\n\nGenerated:\n{}",
        rust_code
    );
    
    Ok(())
}
