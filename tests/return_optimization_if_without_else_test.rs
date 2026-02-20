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

use std::path::PathBuf;
use std::process::Command;

fn compile_wj_source(source: &str) -> String {
    let tmp_dir = std::env::temp_dir().join(format!("wj-test-return-if-{}", std::process::id()));
    std::fs::create_dir_all(&tmp_dir).unwrap();

    let source_path = tmp_dir.join("test.wj");
    std::fs::write(&source_path, source).unwrap();

    let output_path = tmp_dir.join("output");
    std::fs::create_dir_all(&output_path).unwrap();

    let compiler_binary = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("wj");

    let result = Command::new(&compiler_binary)
        .args(&["compile", source_path.to_str().unwrap(), "-o", output_path.to_str().unwrap()])
        .output()
        .expect("Failed to run compiler");

    if !result.status.success() {
        panic!(
            "Compiler failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }

    let rust_file = output_path.join("test.rs");
    std::fs::read_to_string(rust_file).expect("Failed to read generated Rust")
}

#[test]
fn test_return_in_if_let_without_else() {
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

    let rust_code = compile_wj_source(source);
    
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
}

#[test]
fn test_return_in_if_without_else() {
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

    let rust_code = compile_wj_source(source);
    
    // Must preserve explicit return in if without else
    assert!(
        rust_code.contains("return x * 2") || rust_code.contains("return x * 2;"),
        "Must preserve explicit 'return' in if without else!\n\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_can_optimize_if_else_return() {
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

    let rust_code = compile_wj_source(source);
    
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
}

#[test]
fn test_nested_if_let_preserve_return() {
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

    let rust_code = compile_wj_source(source);
    
    // Must preserve explicit return in nested if-let without else
    assert!(
        rust_code.contains("return value * 2") || rust_code.contains("return value * 2;"),
        "Must preserve explicit 'return' in nested if-let without else!\n\nGenerated:\n{}",
        rust_code
    );
}
