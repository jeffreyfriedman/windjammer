//! TDD: Copy semantics in tuple pattern binding (E0614 fix)
//!
//! Bug: let (nx, ny, cost) = vec[i] marked bindings as &i32, &i32, &f32.
//! Rust auto-copies Copy types from references, so bindings are i32, i32, f32 (owned).
//!
//! Fix: infer_match_bound_types returns owned types for Copy tuple elements when
//! destructuring from Index (which yields &T). Also add * to RHS for Copy tuples.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn compile_wj_to_rust(source: &str) -> (String, bool) {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "wj_copy_tuple_{}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        id
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let wj_file = dir.join("test.wj");
    std::fs::write(&wj_file, source).unwrap();

    let _output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_file.to_str().unwrap(),
            "--output",
            dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");

    let src_dir = dir.join("src");
    let main_rs = if src_dir.join("main.rs").exists() {
        src_dir.join("main.rs")
    } else {
        dir.join("test.rs")
    };

    let rs_content = std::fs::read_to_string(&main_rs).unwrap_or_default();

    let rlib_output = dir.join("test.rlib");
    let rustc = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            "-o",
            rlib_output.to_str().unwrap(),
        ])
        .arg(&main_rs)
        .output()
        .expect("Failed to run rustc");

    let compiles = rustc.status.success();
    if !compiles {
        eprintln!("rustc stderr:\n{}", String::from_utf8_lossy(&rustc.stderr));
    }

    (rs_content, compiles)
}

#[test]
fn test_tuple_destructure_all_copy_from_ref() {
    let src = r#"
pub fn process(items: Vec<(i32, i32, f32)>) -> i32 {
    let (nx, ny, cost) = items[0]
    nx + ny
}
"#;

    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(
        result.contains("nx + ny"),
        "Expected nx + ny (owned). Got:\n{}",
        result
    );
    assert!(
        !result.contains("*nx + *ny"),
        "Should NOT add * to Copy bindings. Got:\n{}",
        result
    );
}

#[test]
fn test_tuple_destructure_mixed_copy_noncopy() {
    let src = r#"
pub fn process(items: Vec<(i32, String)>) -> i32 {
    let (id, name) = items[0]
    id
}
"#;

    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(
        result.contains("id") && (result.contains("let (id, name)") || result.contains("clone()")),
        "id should be i32 (Copy), name may need clone. Got:\n{}",
        result
    );
}

#[test]
fn test_tuple_destructure_three_elements() {
    let src = r#"
pub fn get_neighbors() -> Vec<(i32, i32, f32)> {
    vec![(0, 0, 1.0)]
}

pub fn process() -> i32 {
    let neighbors = get_neighbors()
    let (x, y, cost) = neighbors[0]
    x + y
}
"#;

    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(result.contains("x + y"), "Expected x + y. Got:\n{}", result);
    assert!(
        !result.contains("*x"),
        "Should NOT add * to Copy x. Got:\n{}",
        result
    );
}

#[test]
fn test_tuple_in_hashmap_key() {
    let src = r#"
use std::collections::HashMap

pub fn lookup(map: HashMap<(i32, i32), f32>, neighbors: Vec<(i32, i32, f32)>) -> f32 {
    let (x, y, cost) = neighbors[0]
    match map.get(&(x, y)) {
        Some(v) => *v,
        None => 0.0
    }
}
"#;

    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(
        result.contains("(x, y)"),
        "Expected (x, y) for HashMap key. Got:\n{}",
        result
    );
    assert!(
        !result.contains("(*x, *y)"),
        "Should NOT use (*x, *y). Got:\n{}",
        result
    );
}

#[test]
fn test_tuple_four_elements() {
    let src = r#"
pub fn process(items: Vec<(i32, i32, i32, i32)>) -> i32 {
    let (a, b, c, d) = items[0]
    a + b + c + d
}
"#;

    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(
        result.contains("a + b + c + d"),
        "Expected a + b + c + d. Got:\n{}",
        result
    );
}

#[test]
fn test_nested_tuple() {
    let src = r#"
pub fn process(items: Vec<((i32, i32), f32)>) -> i32 {
    let ((x, y), cost) = items[0]
    x + y
}
"#;

    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(result.contains("x + y"), "Expected x + y. Got:\n{}", result);
}

#[test]
fn test_tuple_from_owned_vec() {
    let src = r#"
pub fn process() -> i32 {
    let items = vec![(1, 2), (3, 4)]
    let (x, y) = items[0]
    x + y
}
"#;

    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(result.contains("x + y"), "Expected x + y. Got:\n{}", result);
}

#[test]
fn test_tuple_with_bool() {
    let src = r#"
pub fn process(items: Vec<(i32, bool)>) -> i32 {
    let (id, active) = items[0]
    if active { id } else { 0 }
}
"#;

    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(result.contains("if active"), "Expected if active. Got:\n{}", result);
    assert!(
        !result.contains("*active"),
        "Should NOT add * to bool. Got:\n{}",
        result
    );
}

#[test]
fn test_tuple_with_f64() {
    let src = r#"
pub fn process(items: Vec<(i32, f64)>) -> f64 {
    let (id, value) = items[0]
    value
}
"#;

    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(result.contains("value"), "Expected value. Got:\n{}", result);
    assert!(
        !result.contains("*value"),
        "Should NOT add * to f64. Got:\n{}",
        result
    );
}

#[test]
fn test_tuple_single_element() {
    let src = r#"
pub fn process(items: Vec<(i32,)>) -> i32 {
    let (x,) = items[0]
    x
}
"#;

    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(result.contains("x"), "Expected x. Got:\n{}", result);
}

#[test]
fn test_tuple_destructure_in_loop() {
    let src = r#"
pub fn sum_all(items: Vec<(i32, i32)>) -> i32 {
    let mut total = 0
    let mut i = 0
    while i < items.len() {
        let (a, b) = items[i]
        total = total + a + b
        i = i + 1
    }
    total
}
"#;

    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(
        result.contains("a + b"),
        "Expected a + b in loop. Got:\n{}",
        result
    );
}

#[test]
fn test_tuple_destructure_with_usize() {
    let src = r#"
pub fn process(items: Vec<(usize, i32)>) -> i32 {
    let (idx, val) = items[0]
    val
}
"#;

    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(result.contains("val"), "Expected val. Got:\n{}", result);
}

#[test]
fn test_tuple_destructure_in_arithmetic() {
    let src = r#"
pub fn process(items: Vec<(f32, f32)>) -> f32 {
    let (x, y) = items[0]
    x * x + y * y
}
"#;

    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(
        result.contains("x * x") || result.contains("y * y"),
        "Expected arithmetic. Got:\n{}",
        result
    );
}

#[test]
fn test_tuple_destructure_passed_to_fn() {
    let src = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn process(items: Vec<(i32, i32)>) -> i32 {
    let (x, y) = items[0]
    add(x, y)
}
"#;

    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(
        result.contains("add(x, y)") || result.contains("add(x,y)"),
        "Expected add(x, y). Got:\n{}",
        result
    );
}

#[test]
fn test_tuple_destructure_with_match() {
    let src = r#"
pub fn process(items: Vec<(i32, i32)>) -> i32 {
    let (x, y) = items[0]
    match x {
        0 => y,
        _ => x + y
    }
}
"#;

    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(
        result.contains("x + y") || result.contains("y"),
        "Expected match arms. Got:\n{}",
        result
    );
}
