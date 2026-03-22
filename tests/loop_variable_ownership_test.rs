/// TDD Test: Loop variable ownership
///
/// Loop variables should be owned (i32, T) not borrowed (&i32, &T) when iterating over:
/// - Ranges: for i in 0..10, for x in min..max
/// - Owned collections: for item in vec (consumes vec, yields T)
///
/// Root cause: Loop variables from range iteration were inferred as &i32,
/// causing "casting &i32 as usize is invalid" when used in tiles[row as usize].
///
/// Related errors:
/// - E0606: casting &i32 as usize is invalid
/// - E0277: binary operation on &i32

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

/// for i in 0..10 - i should be i32 (owned)
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_i_in_literal_range() {
    let src = r#"
pub fn sum_ten() -> i32 {
    let mut total = 0
    for i in 0..10 {
        total = total + i
    }
    total
}
"#;

    let result = compile_to_rust(src).expect("compile");
    // Loop variable i should be used directly (no *i)
    // Accept any form: total + i, total = total + i, or total += i
    assert!(
        result.contains("total + i") || result.contains("total = total + i") || result.contains("total += i"),
        "Should use i directly in arithmetic, got: {}",
        result
    );
    assert!(
        !result.contains("*i"),
        "Should not dereference loop variable i, got: {}",
        result
    );
}

/// for x in min..max - x should be i32 (owned)
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_x_in_param_range() {
    let src = r#"
pub fn iterate_range(min: i32, max: i32) -> i32 {
    let mut sum = 0
    for x in min..max {
        sum = sum + x
    }
    sum
}
"#;

    let result = compile_to_rust(src).expect("compile");
    assert!(
        result.contains("sum + x") || result.contains("sum = sum + x") || result.contains("sum += x"),
        "Should use x directly, got: {}",
        result
    );
    assert!(
        !result.contains("*x"),
        "Should not dereference loop variable x, got: {}",
        result
    );
}

/// for item in vec - item should be owned T (not &T) when vec is consumed
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_item_in_vec() {
    let src = r#"
pub fn sum_vec(items: Vec<i32>) -> i32 {
    let mut total = 0
    for item in items {
        total = total + item
    }
    total
}
"#;

    let result = compile_to_rust(src).expect("compile");
    assert!(
        result.contains("total + item") || result.contains("total = total + item") || result.contains("total += item"),
        "Should use item directly, got: {}",
        result
    );
}

/// Loop variable used in arithmetic: i * 2
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_loop_var_in_arithmetic() {
    let src = r#"
pub fn double_range(max: i32) -> i32 {
    let mut sum = 0
    for i in 0..max {
        sum = sum + i * 2
    }
    sum
}
"#;

    let result = compile_to_rust(src).expect("compile");
    assert!(
        result.contains("i * 2"),
        "Should multiply i directly, got: {}",
        result
    );
    assert!(
        !result.contains("*i"),
        "Should not dereference i, got: {}",
        result
    );
}

/// Loop variable used in casting: i as usize - THE KEY TEST
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_loop_var_in_cast() {
    let src = r#"
pub fn index_tiles(tiles: Vec<i32>, min: i32, max: i32) -> i32 {
    let mut sum = 0
    for row in min..max {
        let tile = tiles[row as usize]
        sum = sum + tile
    }
    sum
}
"#;

    let result = compile_to_rust(src).expect("compile");
    // Critical: row as usize, NOT *row as usize (which would mean row was &i32)
    assert!(
        result.contains("row as usize"),
        "Should cast row directly to usize, got: {}",
        result
    );
    assert!(
        !result.contains("*row"),
        "Should not dereference row (would indicate &i32 bug), got: {}",
        result
    );
}

/// Loop variable used in function call
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_loop_var_in_function_call() {
    let src = r#"
pub fn process_range(min: i32, max: i32) {
    for i in min..max {
        println!("{}", i)
    }
}
"#;

    let result = compile_to_rust(src).expect("compile");
    assert!(
        result.contains("println!") && result.contains(", i)"),
        "Should pass i to println, got: {}",
        result
    );
}
