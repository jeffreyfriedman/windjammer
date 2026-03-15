//! Ownership-Based Deref Tests
//!
//! TDD for ownership tracker migration. Replaces borrowed_iterator_vars guessing
//! with systematic ownership-based decisions. Fixes E0614 permanently.

use std::process::Command;

fn compile_wj_to_rust(source: &str) -> (String, bool) {
    let dir = std::env::temp_dir().join(format!(
        "wj_ownership_deref_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
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

// === Struct Literal Field Tests ===

#[test]
fn test_no_deref_for_owned_expression() {
    let src = r#"
pub struct Point { x: i32, y: i32 }
pub fn make() -> Point {
    let x = 5
    Point { x: x, y: 10 }
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(result.contains("x: x") || result.contains("x: 5"), "Owned x, no *");
    assert!(!result.contains("x: *x"), "Should NOT add * for owned");
}

#[test]
fn test_deref_for_borrowed_copy_in_struct() {
    let src = r#"
pub struct Point { x: i32, y: i32 }
pub fn copy_point(p: &Point) -> Point {
    Point { x: p.x, y: p.y }
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    // p.x on &Point yields i32 (Copy) - may or may not need *
    assert!(!result.contains("**p"), "No double deref");
}

#[test]
fn test_no_double_deref() {
    let src = r#"
pub fn process(values: &Vec<i32>, i: usize) -> i32 {
    values[i]
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(!result.contains("**values"), "No double deref");
}

#[test]
fn test_clone_result_is_owned() {
    let src = r#"
pub struct Data { value: String }
pub fn make(d: &Data) -> Data {
    Data { value: d.value.clone() }
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(result.contains("d.value.clone()"));
    assert!(!result.contains("*d.value.clone()"), "clone() returns owned");
}

// === For-Loop Ownership Tests ===

#[test]
fn test_owned_loop_var_no_deref() {
    let src = r#"
@derive(Copy, Clone)
pub struct Id { v: i32 }
pub fn process(id: Id) {}
pub fn run(ids: Vec<Id>) {
    for id in ids {
        process(id)
    }
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(!result.contains("*(id)"), "Owned loop var, no *");
}

#[test]
fn test_borrowed_loop_var_deref_copy() {
    let src = r#"
pub fn process(x: i32) {}
pub fn run(nums: &Vec<i32>) {
    for x in nums {
        process(x)
    }
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    // x from &Vec<i32> is &i32, need * for process(i32)
    assert!(result.contains("process(") && (result.contains("*x") || result.contains("process(x)")), "Borrowed Copy");
}

// === Binary Operation Tests ===

#[test]
fn test_binary_op_owned_operands() {
    let src = r#"
pub fn compute() -> i32 {
    let a = 5
    let b = 10
    a + b
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(result.contains("a + b") || result.contains("5 + 10"));
}

#[test]
fn test_binary_op_mixed_ownership() {
    let src = r#"
pub fn add_one(r: &i32, y: i32) -> i32 {
    r + y
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    // &i32 + i32 needs *r
    assert!(!result.contains("r + y") || result.contains("*r"), "Deref borrowed");
}

// === Function Argument Tests ===

#[test]
fn test_arg_owned_to_owned_param() {
    let src = r#"
pub fn take(x: i32) {}
pub fn call() {
    let x = 42
    take(x)
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(result.contains("take(x)") || result.contains("take(42)"));
}

#[test]
fn test_arg_borrowed_to_owned_param() {
    let src = r#"
pub fn take(x: i32) {}
pub fn call(r: &i32) {
    take(r)
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(result.contains("take(*") || result.contains("take(r)"));
}

#[test]
fn test_arg_owned_to_borrowed_param() {
    let src = r#"
pub fn double(x: &i32) -> i32 {
    x + x
}
pub fn call() {
    let x = 42
    double(x)
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
}

// === Match/If-Let Tests ===

#[test]
fn test_match_option_some_owned() {
    let src = r#"
pub fn process(x: i32) {}
pub fn run(opt: Option<i32>) {
    match opt {
        Some(x) => process(x),
        None => {}
    }
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(!result.contains("*(x)"), "x from Some(x) is owned");
}

#[test]
fn test_match_index_borrowed() {
    let src = r#"
pub struct Node { id: u32 }
pub fn copy_node(n: &Node) -> Node {
    Node { id: n.id }
}
pub fn run(nodes: Vec<Node>) -> Vec<Node> {
    let mut out = Vec::new()
    let mut i = 0
    while i < nodes.len() {
        let n = nodes[i]
        out.push(copy_node(n))
        i = i + 1
    }
    out
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
}

// === Vec Push Tests ===

#[test]
fn test_push_owned_value() {
    let src = r#"
pub fn build() -> Vec<i32> {
    let mut v = Vec::new()
    let x = 42
    v.push(x)
    v
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
}

#[test]
fn test_push_borrowed_needs_clone() {
    let src = r#"
pub fn build(items: &Vec<String>) -> Vec<String> {
    let mut out = Vec::new()
    for item in items {
        out.push(item)
    }
    out
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(result.contains(".clone()"), "Borrowed needs clone for push");
}

// === Field Access Tests ===

#[test]
fn test_field_access_copy_on_borrowed() {
    let src = r#"
pub struct Vec2 { x: f32, y: f32 }
pub fn length(v: &Vec2) -> f32 {
    v.x * v.x + v.y * v.y
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
}

#[test]
fn test_field_access_non_copy_on_borrowed() {
    let src = r#"
pub struct Data { name: String }
pub fn get_name(d: &Data) -> String {
    d.name
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(result.contains(".clone()") || result.contains("d.name"), "Need clone or move");
}

// === E0614 Regression Tests ===

#[test]
fn test_entity_copy_no_deref() {
    let src = r#"
@derive(Copy, Clone, Debug)
pub struct Entity { index: i64 }
pub fn process(e: Entity) {}
pub fn run(entities: Vec<Entity>) {
    for entity in entities {
        process(entity)
    }
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(!result.contains("*(entity)"), "E0614: no * for owned Copy");
}

#[test]
fn test_tuple_pattern_copy_no_deref() {
    let src = r#"
@derive(Copy, Clone)
pub struct Id { v: i32 }
pub fn process(id: Id, x: i32) {}
pub fn run(pairs: Vec<(Id, i32)>) {
    for (id, x) in pairs {
        process(id, x)
    }
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(!result.contains("*(id)"), "Tuple pattern Copy, no *");
}

// === Additional Coverage ===

#[test]
fn test_method_call_owned_receiver() {
    let src = r#"
pub struct Counter { v: i32 }
pub fn get(c: Counter) -> i32 {
    c.v
}
pub fn main() {
    let c = Counter { v: 1 }
    get(c)
}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
}

#[test]
fn test_method_call_borrowed_receiver() {
    let src = r#"
pub struct Counter { v: i32 }
pub fn get(c: &Counter) -> i32 {
    c.v
}
pub fn main() {
    let c = Counter { v: 1 }
    get(c)
}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
}

#[test]
fn test_some_owned_value() {
    let src = r#"
pub fn wrap(x: i32) -> Option<i32> {
    Some(x)
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
}

#[test]
fn test_some_borrowed_value() {
    let src = r#"
pub fn wrap(s: &String) -> Option<String> {
    Some(s)
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(result.contains(".clone()") || result.contains(".to_string()"), "Borrowed to owned");
}

#[test]
fn test_index_copy_element() {
    let src = r#"
pub fn first(nums: &Vec<i32>) -> i32 {
    nums[0]
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
}

#[test]
fn test_assert_eq_owned() {
    let src = r#"
pub fn check() {
    let x = 5
    let y = 5
    assert_eq(x, y)
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
}

#[test]
fn test_assert_eq_borrowed() {
    let src = r#"
pub fn check(r: &i32, y: i32) {
    assert_eq(r, y)
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
}

#[test]
fn test_tuple_literal_owned() {
    let src = r#"
pub fn make() -> (i32, i32) {
    let a = 1
    let b = 2
    (a, b)
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
}

#[test]
fn test_struct_with_borrowed_field_access() {
    let src = r#"
pub struct Inner { x: i32 }
pub struct Outer { inner: Inner }
pub fn get_x(o: &Outer) -> i32 {
    o.inner.x
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
}

#[test]
fn test_explicit_deref_stripped_for_copy() {
    let src = r#"
@derive(Copy, Clone)
pub struct Id { v: i32 }
pub fn collect(ids: Vec<Id>) -> Vec<Id> {
    let mut out = Vec::new()
    for id in ids {
        out.push(*id)
    }
    out
}
pub fn main() {}
"#;
    let (result, compiles) = compile_wj_to_rust(src);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(!result.contains("push(*id)"), "Strip * for owned Copy");
}
