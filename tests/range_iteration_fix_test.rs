/// TDD Test: Range iteration with borrowed variables
///
/// Root cause: for-loop ranges like `min..max` should NOT have borrowed bounds
/// Example: `for i in &min..&max` is wrong, should be `for i in min..max`
///
/// Related errors:
/// - E0277: Range<&i32> is not an iterator
/// - E0308: expected &i32, found i32 in range
/// - E0606: casting &i32 as usize is invalid

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_to_rust(code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, code).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(stderr.to_string());
    }

    let generated_path = out_dir.join("test.rs");
    fs::read_to_string(&generated_path).map_err(|e| e.to_string())
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_range_for_loop_no_borrow() {
    let source = r#"
pub fn iterate_range(min: i32, max: i32) {
    for i in min..max {
        println!("{}", i)
    }
}
"#;

    let rust_code = compile_to_rust(source).expect("compile");

    // Should generate: for i in min..max (NOT &min..&max)
    assert!(
        rust_code.contains("for i in min..max"),
        "Range bounds should not be borrowed, got: {}",
        rust_code
    );
    assert!(
        !rust_code.contains("&min.."),
        "Should not borrow range start, got: {}",
        rust_code
    );
    assert!(
        !rust_code.contains("..&max"),
        "Should not borrow range end, got: {}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_range_for_loop_with_arithmetic() {
    let source = r#"
pub fn iterate_calculated(start: i32, end: i32, size: i32) {
    for i in start.max(0)..end + 1.min(size) {
        let index = i as usize
    }
}
"#;

    let rust_code = compile_to_rust(source).expect("compile");

    // Range bounds should be values, not references
    assert!(rust_code.contains("for i in "), "Has for-loop, got: {}", rust_code);
    assert!(!rust_code.contains("&start"), "Start not borrowed, got: {}", rust_code);
    assert!(
        !rust_code.contains("..&end"),
        "End not borrowed in range, got: {}",
        rust_code
    );

    // Cast should work (no & in front)
    assert!(
        rust_code.contains("i as usize"),
        "Should be able to cast i directly, got: {}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_loop_variable_is_owned() {
    let source = r#"
pub fn use_loop_var(max: i32) {
    for i in 0..max {
        let doubled = i * 2
        let index = i as usize
    }
}
"#;

    let rust_code = compile_to_rust(source).expect("compile");

    // Loop variable should be owned (i32), not borrowed (&i32)
    // So we can multiply it and cast it without dereferencing
    assert!(
        rust_code.contains("i * 2"),
        "Should multiply i directly, got: {}",
        rust_code
    );
    assert!(
        rust_code.contains("i as usize"),
        "Should cast i directly, got: {}",
        rust_code
    );
    assert!(
        !rust_code.contains("*i"),
        "Should not need to dereference loop variable, got: {}",
        rust_code
    );
}
